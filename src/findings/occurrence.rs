use crate::findings::types::Finding;
use crate::scan::cache::stable_hash_hex;

/// Disambiguates findings that share a stable `id`.
///
/// `Finding::id` (see `crate::baseline::key::stable_finding_key`) is
/// intentionally tolerant of line moves and literal-value churn so it
/// survives across scans as a baseline key — which means it can, and in real
/// repos does, collide across genuinely distinct occurrences (two secrets on
/// different lines, two SQL params in the same file). `occurrence_key` hashes
/// the finding's *exact* evidence instead — no normalization, no masking —
/// so two occurrences that share an `id` but differ in location or snippet
/// always get different keys.
///
/// Not used for baseline matching (which must stay keyed on `id` alone) —
/// this is report-local disambiguation only, computed fresh at render time,
/// never persisted.
pub fn occurrence_key(finding: &Finding) -> String {
    let mut parts = vec![finding.rule_id.as_str().to_string()];
    if finding.evidence.is_empty() {
        parts.push(finding.title.clone());
    }
    for evidence in &finding.evidence {
        parts.push(evidence.path.display().to_string());
        parts.push(evidence.line_start.to_string());
        parts.push(
            evidence
                .line_end
                .map(|line| line.to_string())
                .unwrap_or_default(),
        );
        parts.push(evidence.snippet.clone());
    }
    stable_hash_hex(parts.join("\u{1}").as_bytes())
        .chars()
        .take(16)
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::findings::provenance::FindingProvenance;
    use crate::findings::severity::Severity;
    use crate::findings::types::{Confidence, Evidence, FindingCategory};
    use std::path::PathBuf;

    fn finding_at(path: &str, line_start: usize, snippet: &str) -> Finding {
        Finding {
            id: "rule.example:some/path.rs:deadbeef".to_string(),
            rule_id: "rule.example".to_string(),
            title: "Example finding".to_string(),
            description: "desc".to_string(),
            recommendation: String::new(),
            category: FindingCategory::Security,
            severity: Severity::Medium,
            confidence: Confidence::Medium,
            evidence: vec![Evidence {
                path: PathBuf::from(path),
                line_start,
                line_end: None,
                snippet: snippet.to_string(),
            }],
            workspace_package: None,
            docs_url: None,
            provenance: FindingProvenance::default(),
            risk: Default::default(),
        }
    }

    #[test]
    fn same_stable_id_different_location_gets_different_occurrence_key() {
        // Same normalized identity (masked digits/strings) but different raw
        // line numbers, mirroring the real colliding-secret scenario.
        let a = finding_at("src/lib.rs", 10, "const API_KEY: &str = \"abc123\";");
        let b = finding_at("src/lib.rs", 20, "const API_KEY: &str = \"xyz789\";");
        assert_ne!(occurrence_key(&a), occurrence_key(&b));
    }

    #[test]
    fn identical_evidence_produces_identical_occurrence_key() {
        let a = finding_at("src/lib.rs", 10, "same snippet");
        let b = finding_at("src/lib.rs", 10, "same snippet");
        assert_eq!(occurrence_key(&a), occurrence_key(&b));
    }

    #[test]
    fn empty_evidence_falls_back_to_title_without_panicking() {
        let mut finding = finding_at("src/lib.rs", 10, "snippet");
        finding.evidence.clear();
        let key = occurrence_key(&finding);
        assert_eq!(key.len(), 16);
    }
}
