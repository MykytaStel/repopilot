pub(crate) fn render_risk_summary(output: &mut String, summary: &ScanSummary, stats: &ReportStats) {
    output.push_str("## Risk Summary\n\n");

    if summary.artifacts.findings.is_empty() {
        output.push_str("No findings found.\n\n");
        return;
    }

    writeln!(
        output,
        "- **Severity:** {}",
        markdown_severity_counts_text(stats)
    )
    .unwrap();
    writeln!(
        output,
        "- **Categories:** {}",
        named_counts_text(&stats.category_counts)
    )
    .unwrap();
    if !stats.top_paths.is_empty() {
        writeln!(
            output,
            "- **Top paths:** {}",
            named_counts_text(&stats.top_paths)
        )
        .unwrap();
    }
    if !stats.top_packages.is_empty() {
        writeln!(
            output,
            "- **Top packages:** {}",
            named_counts_text(&stats.top_packages)
        )
        .unwrap();
    }
    output.push('\n');
}

pub(crate) fn render_signal_quality(output: &mut String, summary: &ScanSummary) {
    let quality = &summary.artifacts.signal_quality;
    output.push_str("## Signal Quality\n\n");
    if summary.metrics.raw_findings_count > summary.metrics.visible_findings_count {
        writeln!(
            output,
            "- **Visible findings:** {}",
            summary.metrics.visible_findings_count
        )
        .unwrap();
        writeln!(
            output,
            "- **Raw findings:** {}",
            summary.metrics.raw_findings_count
        )
        .unwrap();
        writeln!(
            output,
            "- **Raw high confidence:** {}",
            summary.artifacts.raw_signal_quality.by_confidence.high
        )
        .unwrap();
    }
    writeln!(
        output,
        "- **High confidence:** {}",
        quality.by_confidence.high
    )
    .unwrap();
    writeln!(
        output,
        "- **Medium confidence:** {}",
        quality.by_confidence.medium
    )
    .unwrap();
    writeln!(
        output,
        "- **Low confidence:** {}",
        quality.by_confidence.low
    )
    .unwrap();
    writeln!(
        output,
        "- **Evidence coverage:** {}%",
        quality.evidence_coverage_percent
    )
    .unwrap();
    writeln!(
        output,
        "- **Recommendation coverage:** {}%",
        quality.recommendation_coverage_percent
    )
    .unwrap();
    writeln!(
        output,
        "- **Contract warnings:** {}",
        quality.contract_violations
    )
    .unwrap();
    output.push('\n');
}

pub(crate) fn render_top_rules(output: &mut String, stats: &ReportStats) {
    output.push_str("## Top Rules\n\n");

    if stats.top_rules.is_empty() {
        output.push_str("No rules triggered.\n\n");
        return;
    }

    output.push_str("| Rule | Count | Max severity |\n");
    output.push_str("| --- | ---: | --- |\n");
    for rule in &stats.top_rules {
        let severity = rule
            .severity
            .map(|severity| severity.label())
            .unwrap_or("INFO");
        writeln!(
            output,
            "| `{}` | {} | {} |",
            escape_table_cell(&rule.label),
            rule.count,
            severity
        )
        .unwrap();
    }
    output.push('\n');
}

pub(crate) fn render_top_risk_clusters(output: &mut String, findings: &[Finding]) {
    output.push_str("## Top Risk Clusters\n\n");

    if findings.is_empty() {
        output.push_str("No risk clusters found.\n\n");
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
    clusters.truncate(8);

    output.push_str("| Priority | Area | Rule | Count | Max severity | Max risk | Examples |\n");
    output.push_str("| --- | --- | --- | ---: | --- | ---: | --- |\n");
    for cluster in clusters {
        let area = cluster.scope.as_deref().unwrap_or(".");
        let examples = example_locations(&cluster.findings, 2).join(", ");
        writeln!(
            output,
            "| {} | `{}` | `{}` | {} | {} | {} | {} |",
            cluster.priority.label(),
            escape_table_cell(area),
            escape_table_cell(cluster.rule_id),
            cluster.findings.len(),
            cluster.severity.label(),
            cluster.max_score,
            escape_table_cell(&examples)
        )
        .unwrap();
    }
    output.push('\n');
}
