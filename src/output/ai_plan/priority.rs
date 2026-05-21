fn priority_label(priority: u8) -> &'static str {
    match priority {
        0 => "P0 - Immediate risk",
        1 => "P1 - High-impact hardening",
        2 => "P2 - Quality and maintainability",
        _ => "P3 - Backlog cleanup",
    }
}

fn sort_ai_plan_clusters(clusters: &mut [RuleCluster<'_>]) {
    clusters.sort_by(|left, right| {
        priority_rank(left)
            .cmp(&priority_rank(right))
            .then_with(|| right.max_score.cmp(&left.max_score))
            .then_with(|| cluster_priority(left).cmp(&cluster_priority(right)))
            .then_with(|| right.severity.cmp(&left.severity))
            .then_with(|| cluster_category_rank(left).cmp(&cluster_category_rank(right)))
            .then_with(|| right.findings.len().cmp(&left.findings.len()))
            .then_with(|| left.rule_id.cmp(right.rule_id))
    });
}

fn cluster_priority(cluster: &RuleCluster<'_>) -> u8 {
    cluster
        .findings
        .iter()
        .map(|finding| legacy_priority_rank(finding))
        .min()
        .unwrap_or(3)
}

fn priority_rank(cluster: &RuleCluster<'_>) -> u8 {
    cluster.priority.rank()
}

fn legacy_priority_rank(finding: &Finding) -> u8 {
    if finding.severity == Severity::Critical
        || (finding.severity == Severity::High && finding.category == FindingCategory::Security)
    {
        0
    } else if finding.severity == Severity::High {
        1
    } else if finding.severity == Severity::Medium {
        2
    } else {
        3
    }
}

fn cluster_category_rank(cluster: &RuleCluster<'_>) -> u8 {
    cluster
        .findings
        .first()
        .map(|finding| category_rank(&finding.category))
        .unwrap_or(u8::MAX)
}
