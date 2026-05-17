use crate::findings::types::Finding;
use crate::frameworks::{DetectedFramework, FrameworkProject};
use crate::output::finding_helpers::{clusters_by_rule_scope, example_locations};
use crate::output::render_helpers::escape_table_cell;
use crate::output::report_stats::{ReportStats, TOOL_VERSION};
use crate::output::report_text::{markdown_severity_counts_text, named_counts_text};
use crate::risk::RiskPriority;
use crate::scan::types::ScanSummary;
use std::fmt::Write;

pub(crate) fn render_overview(output: &mut String, summary: &ScanSummary, stats: &ReportStats) {
    output.push_str("## Overview\n\n");
    writeln!(output, "- **RepoPilot version:** {TOOL_VERSION}").unwrap();
    writeln!(output, "- **Path:** `{}`", summary.root_path.display()).unwrap();
    writeln!(output, "- **Risk:** {}", stats.risk_label).unwrap();
    writeln!(output, "- **Health score:** {}/100", stats.health_score).unwrap();
    writeln!(
        output,
        "- **Findings:** {} ({:.1}/kloc)",
        stats.total_findings, stats.finding_density
    )
    .unwrap();
    writeln!(output, "- **Files analyzed:** {}", summary.files_count).unwrap();
    writeln!(
        output,
        "- **Directories analyzed:** {}",
        summary.directories_count
    )
    .unwrap();
    writeln!(output, "- **Lines of code:** {}", summary.lines_of_code).unwrap();
    if summary.scan_duration_us > 0 {
        writeln!(
            output,
            "- **Scan time:** {:.2}s",
            summary.scan_duration_us as f64 / 1_000_000.0
        )
        .unwrap();
    }
    if summary.skipped_files_count > 0 {
        writeln!(
            output,
            "- **Files skipped:** {} ({} bytes)",
            summary.skipped_files_count, summary.skipped_bytes
        )
        .unwrap();
    }
    output.push('\n');
}

pub(crate) fn render_risk_summary(output: &mut String, summary: &ScanSummary, stats: &ReportStats) {
    output.push_str("## Risk Summary\n\n");

    if summary.findings.is_empty() {
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
        priority_rank(left.priority)
            .cmp(&priority_rank(right.priority))
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

pub(crate) fn render_languages_section(output: &mut String, summary: &ScanSummary) {
    output.push_str("## Languages\n\n");

    if summary.languages.is_empty() {
        output.push_str("No languages detected.\n\n");
        return;
    }

    output.push_str("| Language | Files |\n");
    output.push_str("| --- | ---: |\n");
    for language in &summary.languages {
        writeln!(
            output,
            "| {} | {} |",
            escape_table_cell(&language.name),
            language.files_count
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
    output.push_str("## Frameworks\n\n");
    writeln!(output, "{}\n", labels.join(" | ")).unwrap();
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

    output.push_str("## Framework Projects\n\n");
    output.push_str("| Path | Frameworks |\n");
    output.push_str("| --- | --- |\n");
    for project in nested_projects {
        let labels: Vec<String> = project.frameworks.iter().map(|f| f.label()).collect();
        writeln!(
            output,
            "| `{}` | {} |",
            project.path.display(),
            escape_table_cell(&labels.join(", "))
        )
        .unwrap();
    }
    output.push('\n');
}

fn priority_rank(priority: RiskPriority) -> u8 {
    match priority {
        RiskPriority::P0 => 0,
        RiskPriority::P1 => 1,
        RiskPriority::P2 => 2,
        RiskPriority::P3 => 3,
    }
}
