use crate::findings::types::{Confidence, Evidence, Finding, Severity};
use std::collections::{BTreeMap, BTreeSet};

/// Collapse exact duplicate finding emissions.
///
/// `Finding::id` is a stable baseline key, not an occurrence identity: it is
/// intentionally tolerant of line moves and literal-value churn. Using it as a
/// generic aggregation key would merge distinct real occurrences that merely
/// collide under baseline normalization. This aggregation therefore only merges
/// findings that point at the same rule and exact primary evidence location and
/// have compatible metadata.
pub fn aggregate_duplicate_findings(findings: &mut Vec<Finding>) -> usize {
    let original_len = findings.len();
    let mut aggregated: Vec<Finding> = Vec::with_capacity(findings.len());
    let mut seen_keys: BTreeMap<AggregationKey, Vec<usize>> = BTreeMap::new();

    for finding in findings.drain(..) {
        let Some(key) = aggregation_key(&finding) else {
            aggregated.push(finding);
            continue;
        };

        if let Some(indices) = seen_keys.get(&key)
            && let Some(index) = indices
                .iter()
                .copied()
                .find(|index| findings_are_compatible(&aggregated[*index], &finding))
        {
            merge_finding(&mut aggregated[index], finding);
            continue;
        }

        seen_keys.entry(key).or_default().push(aggregated.len());
        aggregated.push(finding);
    }

    *findings = aggregated;
    original_len.saturating_sub(findings.len())
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
struct AggregationKey {
    rule_id: String,
    path: String,
    line_start: usize,
    line_end: Option<usize>,
    snippet: String,
}

fn aggregation_key(finding: &Finding) -> Option<AggregationKey> {
    let evidence = finding.evidence.first()?;
    Some(AggregationKey {
        rule_id: finding.rule_id.clone(),
        path: normalized_evidence_path(evidence),
        line_start: evidence.line_start,
        line_end: evidence.line_end,
        snippet: evidence.snippet.clone(),
    })
}

fn findings_are_compatible(primary: &Finding, duplicate: &Finding) -> bool {
    primary.rule_id == duplicate.rule_id
        && primary.category == duplicate.category
        && primary.title == duplicate.title
        && primary.description == duplicate.description
        && primary.provenance == duplicate.provenance
        && primary.workspace_package == duplicate.workspace_package
}

fn merge_finding(primary: &mut Finding, duplicate: Finding) {
    primary.severity = max_severity(primary.severity, duplicate.severity);
    primary.confidence = max_confidence(primary.confidence, duplicate.confidence);

    if primary.recommendation.trim().is_empty() && !duplicate.recommendation.trim().is_empty() {
        primary.recommendation = duplicate.recommendation;
    }
    if primary.docs_url.is_none() {
        primary.docs_url = duplicate.docs_url;
    }

    append_distinct_evidence(&mut primary.evidence, duplicate.evidence);
}

fn append_distinct_evidence(existing: &mut Vec<Evidence>, incoming: Vec<Evidence>) {
    let mut seen = existing.iter().map(evidence_key).collect::<BTreeSet<_>>();
    for evidence in incoming {
        if seen.insert(evidence_key(&evidence)) {
            existing.push(evidence);
        }
    }
}

fn evidence_key(evidence: &Evidence) -> (String, usize, Option<usize>, String) {
    (
        normalized_evidence_path(evidence),
        evidence.line_start,
        evidence.line_end,
        evidence.snippet.clone(),
    )
}

fn normalized_evidence_path(evidence: &Evidence) -> String {
    evidence.path.to_string_lossy().replace('\\', "/")
}

fn max_severity(left: Severity, right: Severity) -> Severity {
    if right > left { right } else { left }
}

fn max_confidence(left: Confidence, right: Confidence) -> Confidence {
    if right > left { right } else { left }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::findings::enrichment::enrich_findings;
    use crate::findings::provenance::FindingProvenance;
    use crate::findings::types::FindingCategory;
    use std::path::{Path, PathBuf};

    fn finding(root: &Path, rule_id: &str, line: usize, snippet: &str) -> Finding {
        Finding {
            id: String::new(),
            rule_id: rule_id.to_string(),
            title: "Duplicate-prone finding".to_string(),
            description: "The same underlying issue was detected.".to_string(),
            recommendation: "Review once.".to_string(),
            category: FindingCategory::CodeQuality,
            severity: Severity::Medium,
            confidence: Confidence::Medium,
            evidence: vec![Evidence {
                path: root.join("src/config.rs"),
                line_start: line,
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
    fn aggregates_exact_duplicate_emissions_after_enrichment() {
        let root = PathBuf::from("/repo");
        let mut first = finding(&root, "test.duplicate", 10, "call().unwrap()");
        let mut second = finding(&root, "test.duplicate", 10, "call().unwrap()");
        first.severity = Severity::Low;
        second.confidence = Confidence::High;
        let mut findings = vec![first, second];
        enrich_findings(&mut findings, &root);

        let removed = aggregate_duplicate_findings(&mut findings);

        assert_eq!(removed, 1);
        assert_eq!(findings.len(), 1);
        assert_eq!(findings[0].severity, Severity::Medium);
        assert_eq!(findings[0].confidence, Confidence::High);
        assert_eq!(findings[0].evidence.len(), 1);
    }

    #[test]
    fn does_not_merge_stable_key_collision_at_different_locations() {
        let root = PathBuf::from("/repo");
        let mut findings = vec![
            finding(
                &root,
                "security.secret-candidate",
                10,
                "const API_KEY_1: &str = \"secret-one\";",
            ),
            finding(
                &root,
                "security.secret-candidate",
                20,
                "const API_KEY_2: &str = \"secret-two\";",
            ),
        ];
        enrich_findings(&mut findings, &root);

        assert_eq!(
            findings[0].id, findings[1].id,
            "baseline keys should collide under normalization for this regression case"
        );

        let removed = aggregate_duplicate_findings(&mut findings);

        assert_eq!(removed, 0);
        assert_eq!(findings.len(), 2);
        assert_eq!(findings[0].evidence[0].line_start, 10);
        assert_eq!(findings[1].evidence[0].line_start, 20);
    }

    #[test]
    fn does_not_merge_findings_without_evidence() {
        let mut findings = vec![
            Finding {
                evidence: vec![],
                ..finding(Path::new("/repo"), "test.duplicate", 10, "same")
            },
            Finding {
                evidence: vec![],
                ..finding(Path::new("/repo"), "test.duplicate", 10, "same")
            },
        ];

        let removed = aggregate_duplicate_findings(&mut findings);

        assert_eq!(removed, 0);
        assert_eq!(findings.len(), 2);
    }

    #[test]
    fn does_not_duplicate_identical_evidence_when_merging() {
        let root = PathBuf::from("/repo");
        let mut findings = vec![
            finding(&root, "test.duplicate", 10, "same"),
            finding(&root, "test.duplicate", 10, "same"),
        ];
        enrich_findings(&mut findings, &root);

        let removed = aggregate_duplicate_findings(&mut findings);

        assert_eq!(removed, 1);
        assert_eq!(findings.len(), 1);
        assert_eq!(findings[0].evidence.len(), 1);
    }

    #[test]
    fn does_not_merge_incompatible_metadata_at_same_location() {
        let root = PathBuf::from("/repo");
        let first = finding(&root, "test.duplicate", 10, "same");
        let mut second = finding(&root, "test.duplicate", 10, "same");
        second.title = "Different detector message".to_string();
        let mut findings = vec![first.clone(), second.clone()];
        enrich_findings(&mut findings, &root);
        findings[0].title = first.title;
        findings[1].title = second.title;

        let removed = aggregate_duplicate_findings(&mut findings);

        assert_eq!(removed, 0);
        assert_eq!(findings.len(), 2);
    }
}
