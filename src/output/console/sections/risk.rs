pub(crate) fn render_risk_summary(output: &mut String, stats: &ReportStats) {
    output.push_str("Risk Summary:\n");
    writeln!(
        output,
        "  Severity: {}",
        console_severity_counts_text(stats)
    )
    .unwrap();
    writeln!(
        output,
        "  Priority: P0 {}, P1 {}, P2 {}, P3 {}{}",
        stats.priority_count(RiskPriority::P0),
        stats.priority_count(RiskPriority::P1),
        stats.priority_count(RiskPriority::P2),
        stats.priority_count(RiskPriority::P3),
        stats
            .highest_priority
            .map(|priority| format!(
                " | highest {} | avg score {}",
                priority.label(),
                stats.average_risk_score
            ))
            .unwrap_or_default()
    )
    .unwrap();
    writeln!(
        output,
        "  Categories: {}",
        named_counts_text(&stats.category_counts)
    )
    .unwrap();
    if !stats.top_paths.is_empty() {
        writeln!(
            output,
            "  Top paths: {}",
            named_counts_text(&stats.top_paths)
        )
        .unwrap();
    }
    if !stats.top_packages.is_empty() {
        writeln!(
            output,
            "  Top packages: {}",
            named_counts_text(&stats.top_packages)
        )
        .unwrap();
    }
    output.push('\n');
}

pub(crate) fn render_signal_quality(output: &mut String, summary: &ScanSummary) {
    let quality = &summary.signal_quality;
    output.push_str("Signal quality:\n");
    writeln!(output, "  High confidence: {}", quality.by_confidence.high).unwrap();
    writeln!(
        output,
        "  Medium confidence: {}",
        quality.by_confidence.medium
    )
    .unwrap();
    writeln!(output, "  Low confidence: {}", quality.by_confidence.low).unwrap();
    writeln!(
        output,
        "  Evidence coverage: {}%",
        quality.evidence_coverage_percent
    )
    .unwrap();
    writeln!(
        output,
        "  Recommendation coverage: {}%",
        quality.recommendation_coverage_percent
    )
    .unwrap();
    writeln!(
        output,
        "  Contract warnings: {}",
        quality.contract_violations
    )
    .unwrap();
    output.push('\n');
}

pub(crate) fn render_top_rules(output: &mut String, stats: &ReportStats) {
    output.push_str("Top Rules:\n");

    if stats.top_rules.is_empty() {
        output.push_str("  No rules triggered\n\n");
        return;
    }

    for rule in &stats.top_rules {
        let severity = rule
            .severity
            .map(|severity| color::severity_label(severity.label()))
            .unwrap_or_else(|| color::severity_label("INFO"));
        writeln!(output, "  {:>4}  [{}] {}", rule.count, severity, rule.label).unwrap();
    }
    output.push('\n');
}

pub(crate) fn render_top_risk_clusters(output: &mut String, findings: &[Finding]) {
    output.push_str("Top Risk Clusters:\n");

    if findings.is_empty() {
        output.push_str("  none\n\n");
        return;
    }

    let finding_refs = findings.iter().collect::<Vec<_>>();
    let mut clusters = clusters_by_rule_scope(&finding_refs);
    clusters.sort_by(|left, right| {
        left.priority
            .rank()
            .cmp(&right.priority.rank())
            .then_with(|| right.max_score.cmp(&left.max_score))
            .then_with(|| right.findings.len().cmp(&left.findings.len()))
            .then_with(|| left.rule_id.cmp(right.rule_id))
            .then_with(|| left.scope.cmp(&right.scope))
    });

    for cluster in clusters.into_iter().take(5) {
        let area = cluster.scope.as_deref().unwrap_or(".");
        let examples = example_locations(&cluster.findings, 2).join(", ");
        writeln!(
            output,
            "  {} risk {:>3}  {:>3} finding(s)  {} in {}  {}",
            cluster.priority.label(),
            cluster.max_score,
            cluster.findings.len(),
            cluster.rule_id,
            area,
            examples
        )
        .unwrap();
    }
    output.push('\n');
}
