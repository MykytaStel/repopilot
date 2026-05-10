use crate::findings::types::Finding;
use crate::scan::types::LanguageSummary;
use std::collections::HashMap;

pub(super) fn sort_findings(findings: &mut [Finding]) {
    findings.sort_by(|a, b| {
        b.severity
            .cmp(&a.severity)
            .then_with(|| a.rule_id.cmp(&b.rule_id))
            .then_with(|| {
                let pa = a.evidence.first().map(|e| e.path.as_path());
                let pb = b.evidence.first().map(|e| e.path.as_path());
                pa.cmp(&pb)
            })
            .then_with(|| {
                let la = a.evidence.first().map(|e| e.line_start).unwrap_or(0);
                let lb = b.evidence.first().map(|e| e.line_start).unwrap_or(0);
                la.cmp(&lb)
            })
    });
}

pub(super) fn build_language_summary(languages: HashMap<String, usize>) -> Vec<LanguageSummary> {
    let mut summary: Vec<LanguageSummary> = languages
        .into_iter()
        .map(|(name, files_count)| LanguageSummary { name, files_count })
        .collect();

    summary.sort_by(|left, right| {
        right
            .files_count
            .cmp(&left.files_count)
            .then_with(|| left.name.cmp(&right.name))
    });

    summary
}
