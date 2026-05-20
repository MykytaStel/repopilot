use crate::findings::types::Finding;
use crate::frameworks::{DetectedFramework, FrameworkProject};
use crate::output::finding_helpers::{clusters_by_rule_scope, example_locations};
use crate::output::render_helpers::escape_table_cell;
use crate::output::report_stats::{ReportStats, TOOL_VERSION};
use crate::output::report_text::{markdown_severity_counts_text, named_counts_text};
use crate::report::quality::build_signal_quality_summary;
use crate::scan::types::{DiagnosticSeverity, ScanSummary};
use std::fmt::Write;

pub(crate) fn render_overview(output: &mut String, summary: &ScanSummary, stats: &ReportStats) {
    output.push_str("## Overview\n\n");
    writeln!(output, "- **RepoPilot version:** {TOOL_VERSION}").unwrap();
    writeln!(output, "- **Path:** `{}`", summary.root_path.display()).unwrap();
    if let Some(profile) = &summary.visibility_profile {
        writeln!(output, "- **Profile:** `{profile}`").unwrap();
    }
    render_scope(output, summary);
    writeln!(output, "- **Risk:** {}", stats.risk_label).unwrap();
    writeln!(output, "- **Health score:** {}/100", stats.health_score).unwrap();
    writeln!(
        output,
        "- **Findings:** {} visible ({:.1}/kloc)",
        stats.total_findings, stats.finding_density
    )
    .unwrap();
    if summary.hidden_suggestions_count > 0 {
        writeln!(
            output,
            "- **Hidden suggestions:** {} strict-only suggestions",
            summary.hidden_suggestions_count
        )
        .unwrap();
        writeln!(
            output,
            "- **Note:** {} strict-only suggestions hidden. Run with `--profile strict` or `--include-maintainability` to view.",
            summary.hidden_suggestions_count
        )
        .unwrap();
    }
    render_hidden_suggestions_breakdown(output, summary);
    render_local_feedback(output, summary);
    render_diagnostics(output, summary);
    writeln!(output, "- **Files analyzed:** {}", summary.files_analyzed).unwrap();
    writeln!(
        output,
        "- **Directories analyzed:** {}",
        summary.directories_count
    )
    .unwrap();
    writeln!(output, "- **Non-empty lines:** {}", summary.non_empty_lines).unwrap();
    if summary.scan_duration_us > 0 {
        writeln!(
            output,
            "- **Scan time:** {:.2}s",
            summary.scan_duration_us as f64 / 1_000_000.0
        )
        .unwrap();
    }
    render_cache_telemetry(output, summary);
    if summary.large_files_skipped > 0 {
        writeln!(
            output,
            "- **Files skipped:** {} ({} bytes)",
            summary.large_files_skipped, summary.skipped_bytes
        )
        .unwrap();
    }
    output.push('\n');
}

fn render_local_feedback(output: &mut String, summary: &ScanSummary) {
    let Some(feedback) = &summary.local_feedback else {
        return;
    };

    let path = feedback
        .feedback_path
        .as_ref()
        .map(|path| path.display().to_string())
        .unwrap_or_else(|| ".repopilot/feedback.yml".to_string());
    writeln!(
        output,
        "- **Local feedback:** {} suppression(s) loaded, {} finding(s) suppressed (`{}`)",
        feedback.suppressions_loaded, feedback.suppressed_findings_count, path
    )
    .unwrap();

    if feedback.unmatched_suppressions_count > 0 {
        writeln!(
            output,
            "- **Local feedback unmatched:** {} suppression(s)",
            feedback.unmatched_suppressions_count
        )
        .unwrap();
    }
    if feedback.invalid_suppressions_count > 0 {
        writeln!(
            output,
            "- **Local feedback invalid:** {} suppression(s)",
            feedback.invalid_suppressions_count
        )
        .unwrap();
    }
}

fn render_diagnostics(output: &mut String, summary: &ScanSummary) {
    if summary.diagnostics.is_empty() {
        return;
    }

    output.push_str("- **Diagnostics:**\n");
    for diagnostic in &summary.diagnostics {
        let path = diagnostic
            .path
            .as_ref()
            .map(|path| format!(" `{}`", path.display()))
            .unwrap_or_default();
        writeln!(
            output,
            "  - `{}` `{}`{}: {}",
            diagnostic_severity_label(diagnostic.severity),
            diagnostic.code,
            path,
            diagnostic.message
        )
        .unwrap();
    }
}

fn diagnostic_severity_label(severity: DiagnosticSeverity) -> &'static str {
    match severity {
        DiagnosticSeverity::Info => "info",
        DiagnosticSeverity::Warning => "warning",
        DiagnosticSeverity::Error => "error",
    }
}

fn render_cache_telemetry(output: &mut String, summary: &ScanSummary) {
    let Some(cache) = &summary.cache_telemetry else {
        return;
    };

    writeln!(
        output,
        "- **Cache:** {} hit(s), {} miss(es), {} skipped ({}% hit rate)",
        cache.hits, cache.misses, cache.skipped, cache.hit_rate_percent
    )
    .unwrap();

    if !cache.changed_file_reasons.is_empty() {
        let reasons = cache
            .changed_file_reasons
            .iter()
            .map(|item| format!("{} ({})", item.reason, item.count))
            .collect::<Vec<_>>()
            .join(", ");
        writeln!(output, "- **Changed file reasons:** {reasons}").unwrap();
    }

    writeln!(
        output,
        "- **Cache timing:** load {}ms; hash {}ms; lookup {}ms; reuse {}ms; miss scan {}ms; write {}ms; est. saved {}",
        cache.timings.load_us / 1000,
        cache.timings.file_hash_us / 1000,
        cache.timings.lookup_us / 1000,
        cache.timings.hit_reuse_us / 1000,
        cache.timings.miss_scan_us / 1000,
        cache.timings.write_us / 1000,
        format_optional_ms(cache.timings.estimated_time_saved_us)
    )
    .unwrap();

    if cache.changed_files.is_empty() {
        return;
    }

    output.push_str("- **Changed file cache decisions:**\n");
    for file in cache.changed_files.iter().take(8) {
        writeln!(
            output,
            "  - `{}`: {} ({}, {})",
            file.path.display(),
            file.cache_status,
            file.change_reason,
            file.cache_reason
        )
        .unwrap();
    }

    if cache.changed_files.len() > 8 {
        writeln!(
            output,
            "  - ... {} more changed file(s)",
            cache.changed_files.len() - 8
        )
        .unwrap();
    }
}

fn format_optional_ms(value: Option<u64>) -> String {
    value
        .map(|value| format!("{}ms", value / 1000))
        .unwrap_or_else(|| "n/a".to_string())
}

fn render_hidden_suggestions_breakdown(output: &mut String, summary: &ScanSummary) {
    if summary.hidden_suggestions.is_empty() {
        return;
    }

    output.push_str(
        "- **Top hidden suggestions:**
",
    );

    for item in summary.hidden_suggestions.iter().take(8) {
        writeln!(
            output,
            "  - `{}` / `{}` / `{}`: {} - {}",
            item.category, item.intent, item.rule_id, item.count, item.reason
        )
        .unwrap();
    }

    if summary.hidden_suggestions.len() > 8 {
        writeln!(
            output,
            "  - ... {} more hidden group(s)",
            summary.hidden_suggestions.len() - 8
        )
        .unwrap();
    }
}

fn render_scope(output: &mut String, summary: &ScanSummary) {
    if summary.mode == crate::scan::types::ScanMode::Changed {
        let base = summary
            .base_ref
            .as_ref()
            .map(|base| format!(" since `{base}`"))
            .unwrap_or_else(|| " against `HEAD`".to_string());
        writeln!(
            output,
            "- **Scope:** changed files{base}; {} changed file(s); repo-level rules skipped",
            summary.changed_files_count
        )
        .unwrap();
        return;
    }

    writeln!(output, "- **Scope:** full scan; repo-level rules included").unwrap();
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

pub(crate) fn render_signal_quality(output: &mut String, summary: &ScanSummary) {
    let quality = build_signal_quality_summary(&summary.findings);
    output.push_str("## Signal Quality\n\n");
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
            language.files_analyzed
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
