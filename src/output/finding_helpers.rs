use crate::findings::types::{Finding, FindingCategory, Severity};
use crate::risk::RiskPriority;
use std::collections::BTreeMap;
use std::path::Component;

pub(crate) struct RuleCluster<'a> {
    pub(crate) title: String,
    pub(crate) rule_id: &'a str,
    pub(crate) severity: Severity,
    pub(crate) priority: RiskPriority,
    pub(crate) max_score: u8,
    pub(crate) scope: Option<String>,
    pub(crate) findings: Vec<&'a Finding>,
}

pub(crate) fn clusters_by_rule_scope<'a>(findings: &'a [&'a Finding]) -> Vec<RuleCluster<'a>> {
    let mut by_scope: BTreeMap<(String, String), Vec<&Finding>> = BTreeMap::new();
    for finding in findings.iter().copied() {
        by_scope
            .entry((
                finding.rule_id.clone(),
                cluster_scope_for_finding(finding).unwrap_or_else(|| ".".to_string()),
            ))
            .or_default()
            .push(finding);
    }

    by_scope
        .into_iter()
        .filter_map(|((_, scope), mut group)| {
            group.sort_by(|left, right| {
                crate::risk::compare_findings(left, right)
                    .then_with(|| finding_location(left).cmp(&finding_location(right)))
            });
            let first = group.first().copied()?;
            Some(build_cluster(first, group, Some(scope)))
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

fn build_cluster<'a>(
    first: &'a Finding,
    group: Vec<&'a Finding>,
    scope: Option<String>,
) -> RuleCluster<'a> {
    let severity = group
        .iter()
        .map(|finding| finding.severity)
        .max()
        .unwrap_or(first.severity);
    let max_score = group
        .iter()
        .map(|finding| finding.risk.score)
        .max()
        .unwrap_or(first.risk.score);
    let priority = group
        .iter()
        .map(|finding| finding.risk.priority)
        .min_by_key(|priority| priority.rank())
        .unwrap_or(first.risk.priority);
    let title = cluster_title(first, group.len(), scope.as_deref());

    RuleCluster {
        title,
        rule_id: first.rule_id.as_str(),
        severity,
        priority,
        max_score,
        scope,
        findings: group,
    }
}

fn cluster_title(finding: &Finding, count: usize, scope: Option<&str>) -> String {
    let base = if count > 1 {
        crate::rules::lookup_rule_metadata(&finding.rule_id)
            .map(|metadata| metadata.title)
            .unwrap_or(finding.rule_id.as_str())
    } else {
        finding.title.as_str()
    };

    match scope {
        Some(scope) if count > 1 && scope != "." => format!("{base} in {scope}"),
        _ => base.to_string(),
    }
}

fn cluster_scope_for_finding(finding: &Finding) -> Option<String> {
    finding
        .evidence
        .first()
        .map(|evidence| cluster_scope_for_path(&evidence.path))
}

fn cluster_scope_for_path(path: &std::path::Path) -> String {
    let parts = path
        .components()
        .filter_map(|component| match component {
            Component::CurDir => None,
            Component::Normal(value) => Some(value.to_string_lossy().to_string()),
            Component::RootDir | Component::Prefix(_) | Component::ParentDir => None,
        })
        .collect::<Vec<_>>();

    match (parts.first(), parts.get(1)) {
        (Some(first), Some(second)) if second.contains('.') => first.clone(),
        (Some(first), Some(second)) => format!("{first}/{second}"),
        (Some(first), None) => first.clone(),
        _ => ".".to_string(),
    }
}
