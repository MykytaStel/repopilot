use crate::findings::types::{Finding, FindingCategory};

pub fn sort_findings(findings: &mut [Finding]) {
    findings.sort_by(compare_findings);
}

pub fn compare_findings(left: &Finding, right: &Finding) -> std::cmp::Ordering {
    right
        .risk
        .score
        .cmp(&left.risk.score)
        .then_with(|| right.severity.cmp(&left.severity))
        .then_with(|| category_rank(&left.category).cmp(&category_rank(&right.category)))
        .then_with(|| left.rule_id.cmp(&right.rule_id))
        .then_with(|| finding_location_key(left).cmp(&finding_location_key(right)))
}

fn category_rank(category: &FindingCategory) -> usize {
    match category {
        FindingCategory::Security => 0,
        FindingCategory::Architecture => 1,
        FindingCategory::Framework => 2,
        FindingCategory::CodeQuality => 3,
        FindingCategory::Testing => 4,
    }
}

fn finding_location_key(finding: &Finding) -> String {
    finding
        .evidence
        .first()
        .map(|evidence| format!("{}:{}", evidence.path.display(), evidence.line_start))
        .unwrap_or_default()
}
