use crate::findings::types::{Finding, FindingCategory, Severity};
use std::collections::BTreeMap;

pub(crate) struct RuleCluster<'a> {
    pub(crate) title: &'a str,
    pub(crate) rule_id: &'a str,
    pub(crate) severity: Severity,
    pub(crate) findings: Vec<&'a Finding>,
}

pub(crate) fn clusters_by_rule<'a>(findings: &'a [&'a Finding]) -> Vec<RuleCluster<'a>> {
    let mut by_rule: BTreeMap<&str, Vec<&Finding>> = BTreeMap::new();
    for finding in findings.iter().copied() {
        by_rule.entry(&finding.rule_id).or_default().push(finding);
    }

    by_rule
        .into_values()
        .filter_map(|mut group| {
            group.sort_by(|left, right| {
                crate::risk::compare_findings(left, right)
                    .then_with(|| finding_location(left).cmp(&finding_location(right)))
            });
            let first = group.first().copied()?;
            Some(RuleCluster {
                title: cluster_title(first, group.len()),
                rule_id: first.rule_id.as_str(),
                severity: group
                    .iter()
                    .map(|finding| finding.severity)
                    .max()
                    .unwrap_or(first.severity),
                findings: group,
            })
        })
        .collect()
}

pub(crate) fn category_rank(category: &FindingCategory) -> u8 {
    match category {
        FindingCategory::Security => 0,
        FindingCategory::Architecture => 1,
        FindingCategory::Framework => 2,
        FindingCategory::CodeQuality => 3,
        FindingCategory::Testing => 4,
    }
}

pub(crate) fn finding_recommendation(finding: &Finding) -> &str {
    finding.recommendation_or_default()
}

pub(crate) fn finding_location(finding: &Finding) -> Option<String> {
    finding.evidence.first().map(|evidence| {
        let path = evidence.path.display().to_string();
        if evidence.line_start > 0 {
            format!("{path}:{}", evidence.line_start)
        } else {
            path
        }
    })
}

pub(crate) fn finding_location_key(finding: &Finding) -> String {
    finding
        .evidence
        .first()
        .map(|evidence| format!("{}:{}", evidence.path.display(), evidence.line_start))
        .unwrap_or_default()
}

pub(crate) fn example_locations(findings: &[&Finding], limit: usize) -> Vec<String> {
    findings
        .iter()
        .filter_map(|finding| finding_location(finding))
        .take(limit)
        .map(|location| format!("`{location}`"))
        .collect()
}

fn cluster_title(finding: &Finding, count: usize) -> &str {
    if count > 1 {
        crate::rules::lookup_rule_metadata(&finding.rule_id)
            .map(|metadata| metadata.title)
            .unwrap_or(finding.rule_id.as_str())
    } else {
        finding.title.as_str()
    }
}
