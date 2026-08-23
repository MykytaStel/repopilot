pub(super) fn render_summary_cards(summary: &ScanSummary, stats: &ReportStats) -> String {
    let (risk, health, maintainability) = assessment_cards(summary, stats);
    let mut cards = vec![
        summary_card(risk, "Risk"),
        summary_card(health, "Visible health"),
        summary_card(maintainability, "Maintainability"),
        summary_card(stats.total_findings, "Visible Findings"),
        summary_card(summary.metrics.files_analyzed, "Files"),
        summary_card(summary.metrics.non_empty_lines, "Non-empty Lines"),
        summary_card(format!("{:.1}/kloc", stats.finding_density), "Density"),
    ];

    if summary.metrics.hidden_suggestions_count > 0 {
        cards.push(summary_card(
            summary.metrics.hidden_suggestions_count,
            "Hidden Suggestions",
        ));
    }

    if summary.metrics.large_files_skipped > 0 {
        cards.push(summary_card(summary.metrics.large_files_skipped, "Skipped"));
    }

    cards.join("\n  ")
}

pub(super) fn render_baseline_summary_cards(
    report: &BaselineScanReport,
    stats: &ReportStats,
) -> String {
    let (risk, health, maintainability) = assessment_cards(&report.summary, stats);
    let mut cards = vec![
        summary_card(risk, "Risk"),
        summary_card(health, "Visible health"),
        summary_card(maintainability, "Maintainability"),
        summary_card(report.summary.artifacts.findings.len(), "Visible Findings"),
        summary_card(report.new_count(), "New"),
        summary_card(report.existing_count(), "Existing"),
        summary_card(report.summary.metrics.files_analyzed, "Files"),
    ];

    if report.summary.metrics.hidden_suggestions_count > 0 {
        cards.push(summary_card(
            report.summary.metrics.hidden_suggestions_count,
            "Hidden Suggestions",
        ));
    }

    if report.summary.metrics.large_files_skipped > 0 {
        cards.push(summary_card(report.summary.metrics.large_files_skipped, "Skipped"));
    }

    cards.join("\n  ")
}

fn assessment_cards(summary: &ScanSummary, stats: &ReportStats) -> (String, String, String) {
    if summary.assessment_status() == AssessmentStatus::NotAssessed {
        return (
            "not assessed".to_string(),
            "not assessed".to_string(),
            "not assessed".to_string(),
        );
    }
    (
        stats.risk_label.to_string(),
        format!("{}/100", stats.health_score),
        format!("{}/100", stats.maintainability_score),
    )
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
