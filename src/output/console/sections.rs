use crate::findings::types::Finding;
use crate::frameworks::{DetectedFramework, FrameworkProject};
use crate::output::color;
use crate::output::finding_helpers::{clusters_by_rule_scope, example_locations};
use crate::output::report_stats::{ReportStats, TOOL_VERSION};
use crate::output::report_text::{console_severity_counts_text, named_counts_text};
use crate::risk::RiskPriority;
use crate::scan::types::ScanSummary;
use std::fmt::Write;

pub(crate) fn render_header(output: &mut String, summary: &ScanSummary, stats: &ReportStats) {
    output.push_str("RepoPilot Scan\n");
    writeln!(output, "Version: {TOOL_VERSION}").unwrap();
    writeln!(output, "Path: {}", summary.root_path.display()).unwrap();
    writeln!(
        output,
        "Risk: {} | Health score: {}/100 {}",
        stats.risk_label,
        stats.health_score,
        health_score_bar(stats.health_score)
    )
    .unwrap();
    writeln!(
        output,
        "Findings: {} visible ({:.1}/kloc)",
        stats.total_findings, stats.finding_density
    )
    .unwrap();
    if summary.hidden_suggestions_count > 0 {
        writeln!(
            output,
            "Hidden suggestions: {} maintainability/testing",
            summary.hidden_suggestions_count
        )
        .unwrap();
        writeln!(
            output,
            "Note: {} maintainability/testing suggestions hidden. Run with --profile strict to view.",
            summary.hidden_suggestions_count
        )
        .unwrap();
    }
    writeln!(
        output,
        "Directories analyzed: {} | Lines of code: {}",
        summary.directories_count, summary.lines_of_code
    )
    .unwrap();
    render_scan_input(output, summary);
    if summary.scan_duration_us > 0 {
        writeln!(
            output,
            "Scan time: {:.2}s",
            summary.scan_duration_us as f64 / 1_000_000.0
        )
        .unwrap();
    }
    output.push('\n');
}

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
        priority_rank(left.priority)
            .cmp(&priority_rank(right.priority))
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

pub(crate) fn render_languages_section(output: &mut String, summary: &ScanSummary) {
    output.push_str("Languages:\n");

    if summary.languages.is_empty() {
        output.push_str("  No languages detected\n\n");
        return;
    }

    for language in &summary.languages {
        writeln!(
            output,
            "  {}: {} files",
            language.name, language.files_count
        )
        .unwrap();
    }
    output.push('\n');
}

pub(crate) fn render_frameworks_section(output: &mut String, frameworks: &[DetectedFramework]) {
    if frameworks.is_empty() {
        return;
    }
    let labels: Vec<String> = frameworks.iter().map(|f| f.label()).collect();
    writeln!(output, "Frameworks: {}\n", labels.join(" | ")).unwrap();
}

pub(crate) fn render_framework_projects_section(
    output: &mut String,
    projects: &[FrameworkProject],
) {
    let nested_projects: Vec<_> = projects
        .iter()
        .filter(|project| project.path.as_path() != std::path::Path::new("."))
        .collect();
    if nested_projects.is_empty() {
        return;
    }

    output.push_str("Framework projects:\n");
    for project in nested_projects {
        let labels: Vec<String> = project.frameworks.iter().map(|f| f.label()).collect();
        writeln!(
            output,
            "  {}: {}",
            project.path.display(),
            labels.join(" | ")
        )
        .unwrap();
    }
    output.push('\n');
}

fn render_scan_input(output: &mut String, summary: &ScanSummary) {
    output.push_str("Scan input:\n");

    if let Some(path) = &summary.repopilotignore_path {
        writeln!(output, " .repopilotignore: {}", path.display()).unwrap();
    }

    if summary.files_skipped_repopilotignore > 0 {
        writeln!(
            output,
            " Files skipped (.repopilotignore): {:>7}",
            summary.files_skipped_repopilotignore
        )
        .unwrap();
    }

    writeln!(output, " Files discovered: {:>7}", summary.files_discovered).unwrap();

    if summary.files_skipped_by_limit > 0 {
        writeln!(
            output,
            " Files skipped (limit): {:>7}",
            summary.files_skipped_by_limit
        )
        .unwrap();
    }

    writeln!(output, " Files analyzed: {:>7}", summary.files_count).unwrap();

    if summary.skipped_files_count > 0 {
        writeln!(
            output,
            " Large files skipped: {:>7}",
            summary.skipped_files_count
        )
        .unwrap();
    }

    if summary.binary_files_skipped > 0 {
        writeln!(
            output,
            " Binary files skipped: {:>7}",
            summary.binary_files_skipped
        )
        .unwrap();
    }

    if summary.files_skipped_low_signal > 0 {
        writeln!(
            output,
            " Low-signal files skipped:{:>7}",
            summary.files_skipped_low_signal
        )
        .unwrap();
    }
}

fn priority_rank(priority: RiskPriority) -> u8 {
    match priority {
        RiskPriority::P0 => 0,
        RiskPriority::P1 => 1,
        RiskPriority::P2 => 2,
        RiskPriority::P3 => 3,
    }
}

fn health_score_bar(score: u8) -> &'static str {
    match score {
        90..=100 => "[##########] Excellent",
        75..=89 => "[########  ] Good",
        60..=74 => "[######    ] Fair",
        40..=59 => "[####      ] Poor",
        _ => "[##        ] Critical",
    }
}
