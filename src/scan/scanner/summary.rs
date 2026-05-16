use crate::findings::types::Finding;
use crate::scan::types::LanguageSummary;
use std::collections::HashMap;

pub(super) fn sort_findings(findings: &mut [Finding]) {
    crate::risk::sort_findings(findings);
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
