pub(super) fn render_summary_cards(summary: &ScanSummary, stats: &ReportStats) -> String {
    let mut cards = vec![
        summary_card(stats.risk_label, "Risk"),
        summary_card(format!("{}/100", stats.health_score), "Health"),
        summary_card(stats.total_findings, "Visible Findings"),
        summary_card(summary.files_analyzed, "Files"),
        summary_card(summary.non_empty_lines, "Non-empty Lines"),
        summary_card(format!("{:.1}/kloc", stats.finding_density), "Density"),
    ];

    if summary.hidden_suggestions_count > 0 {
        cards.push(summary_card(
            summary.hidden_suggestions_count,
            "Hidden Suggestions",
        ));
    }

    if summary.large_files_skipped > 0 {
        cards.push(summary_card(summary.large_files_skipped, "Skipped"));
    }

    cards.join("\n  ")
}

pub(super) fn render_baseline_summary_cards(
    report: &BaselineScanReport,
    stats: &ReportStats,
) -> String {
    let mut cards = vec![
        summary_card(stats.risk_label, "Risk"),
        summary_card(format!("{}/100", stats.health_score), "Health"),
        summary_card(report.summary.findings.len(), "Visible Findings"),
        summary_card(report.new_count(), "New"),
        summary_card(report.existing_count(), "Existing"),
        summary_card(report.summary.files_analyzed, "Files"),
    ];

    if report.summary.hidden_suggestions_count > 0 {
        cards.push(summary_card(
            report.summary.hidden_suggestions_count,
            "Hidden Suggestions",
        ));
    }

    if report.summary.large_files_skipped > 0 {
        cards.push(summary_card(report.summary.large_files_skipped, "Skipped"));
    }

    cards.join("\n  ")
}

pub(super) fn render_baseline_meta(
    report: &BaselineScanReport,
    ci_gate: Option<&CiGateResult>,
) -> String {
    let baseline = match &report.baseline_path {
        Some(path) => format!(
            "Baseline: <code>{}</code>",
            escape_html(&path.to_string_lossy())
        ),
        None => "Baseline: none (all findings treated as new)".to_string(),
    };
    let gate = ci_gate
        .map(|ci_gate| {
            let status = if ci_gate.passed() { "passed" } else { "failed" };
            format!(" CI gate: {status} ({})", escape_html(&ci_gate.label()))
        })
        .unwrap_or_default();

    format!(r#"<p class="meta">{baseline}.{gate}</p>"#)
}
