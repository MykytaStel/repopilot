use crate::baseline::diff::BaselineScanReport;
use crate::baseline::gate::CiGateResult;
use crate::findings::types::{Finding, Severity};
use crate::frameworks::DetectedFramework;
use crate::frameworks::FrameworkProject;
use crate::frameworks::ReactNativeArchitectureProfile;
use crate::output::render_helpers::{escape_table_cell, workspace_package_counts};
use crate::output::report_stats::{
    ReportStats, TOOL_VERSION, build_report_stats, category_order, findings_for_category,
    findings_for_rule, first_location, rule_ids_for_findings,
};
use crate::output::report_text::{
    category_label_rank, category_title, first_sentence, markdown_severity_counts_text,
    named_counts_text, tristate_label,
};
use crate::scan::types::ScanSummary;
use std::collections::BTreeMap;
use std::fmt::Write;

pub fn render(summary: &ScanSummary) -> String {
    let stats = build_report_stats(summary);
    let mut output = String::new();

    output.push_str("# RepoPilot Scan Report\n\n");
    render_overview(&mut output, summary, &stats);
    render_risk_summary(&mut output, summary, &stats);
    render_top_rules(&mut output, &stats);
    render_languages_section(&mut output, summary);
    render_frameworks_section(&mut output, &summary.detected_frameworks);
    render_framework_projects_section(&mut output, &summary.framework_projects);
    if let Some(rn) = &summary.react_native {
        render_react_native_section(&mut output, rn);
    }
    render_workspace_risk_table(&mut output, &summary.findings);
    render_findings_index(&mut output, &summary.findings, None);
    render_grouped_findings(&mut output, &summary.findings, |_| None);

    output
}

pub fn render_with_baseline(report: &BaselineScanReport, ci_gate: Option<&CiGateResult>) -> String {
    let summary = &report.summary;
    let stats = build_report_stats(summary);
    let mut output = String::new();

    output.push_str("# RepoPilot Scan Report\n\n");
    render_overview(&mut output, summary, &stats);

    output.push_str("## Baseline\n\n");
    match &report.baseline_path {
        Some(path) => writeln!(output, "- **Baseline:** `{}`", path.display()).unwrap(),
        None => output.push_str("- **Baseline:** none (all findings treated as new)\n"),
    }
    writeln!(output, "- **New findings:** {}", report.new_count()).unwrap();
    writeln!(
        output,
        "- **Existing findings:** {}",
        report.existing_count()
    )
    .unwrap();
    if let Some(ci_gate) = ci_gate {
        let status = if ci_gate.passed() { "passed" } else { "failed" };
        writeln!(output, "- **CI gate:** {status} (`{}`)", ci_gate.label()).unwrap();
    }
    output.push('\n');

    render_risk_summary(&mut output, summary, &stats);
    render_top_rules(&mut output, &stats);
    render_languages_section(&mut output, summary);
    render_frameworks_section(&mut output, &summary.detected_frameworks);
    render_framework_projects_section(&mut output, &summary.framework_projects);
    if let Some(rn) = &summary.react_native {
        render_react_native_section(&mut output, rn);
    }
    render_workspace_risk_table(&mut output, &summary.findings);
    render_findings_index(&mut output, &summary.findings, Some(report));
    render_grouped_findings(&mut output, &summary.findings, |index| {
        Some(report.finding_status(index).lowercase_label())
    });

    output
}

fn render_overview(output: &mut String, summary: &ScanSummary, stats: &ReportStats) {
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

fn render_risk_summary(output: &mut String, summary: &ScanSummary, stats: &ReportStats) {
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

fn render_top_rules(output: &mut String, stats: &ReportStats) {
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

fn render_languages_section(output: &mut String, summary: &ScanSummary) {
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

fn render_findings_index(
    output: &mut String,
    findings: &[Finding],
    baseline: Option<&BaselineScanReport>,
) {
    output.push_str("## Findings Index\n\n");

    if findings.is_empty() {
        output.push_str("No findings found.\n\n");
        return;
    }

    let rows = grouped_index_rows(findings, baseline);
    if baseline.is_some() {
        output.push_str(
            "| Category | Rule | Max severity | Count | New | Existing | First location |\n",
        );
        output.push_str("| --- | --- | --- | ---: | ---: | ---: | --- |\n");
        for row in rows {
            writeln!(
                output,
                "| {} | `{}` | {} | {} | {} | {} | {} |",
                escape_table_cell(&row.category),
                escape_table_cell(&row.rule_id),
                row.severity.label(),
                row.count,
                row.new_count,
                row.existing_count,
                escape_table_cell(&row.first_location.unwrap_or_else(|| "n/a".to_string()))
            )
            .unwrap();
        }
    } else {
        output.push_str("| Category | Rule | Max severity | Count | First location |\n");
        output.push_str("| --- | --- | --- | ---: | --- |\n");
        for row in rows {
            writeln!(
                output,
                "| {} | `{}` | {} | {} | {} |",
                escape_table_cell(&row.category),
                escape_table_cell(&row.rule_id),
                row.severity.label(),
                row.count,
                escape_table_cell(&row.first_location.unwrap_or_else(|| "n/a".to_string()))
            )
            .unwrap();
        }
    }
    output.push('\n');
}

fn render_grouped_findings<F>(output: &mut String, findings: &[Finding], status_for: F)
where
    F: Fn(usize) -> Option<&'static str>,
{
    output.push_str("## Findings\n\n");

    if findings.is_empty() {
        output.push_str("No findings found.\n");
        return;
    }

    for category in category_order() {
        let category_findings = findings_for_category(findings, &category);
        if category_findings.is_empty() {
            continue;
        }

        writeln!(output, "### {}\n", category_title(&category)).unwrap();
        let rules = rule_ids_for_findings(&category_findings);
        for rule_id in rules {
            let rule_findings = findings_for_rule(&category_findings, &rule_id);
            writeln!(output, "#### `{rule_id}` ({})\n", rule_findings.len()).unwrap();

            for finding in rule_findings {
                let index = findings
                    .iter()
                    .position(|candidate| std::ptr::eq(candidate, finding))
                    .unwrap_or(0);
                render_finding_detail(output, finding, status_for(index));
            }
        }
    }
}

fn render_finding_detail(output: &mut String, finding: &Finding, status: Option<&str>) {
    writeln!(
        output,
        "- **[{}] {}**",
        finding.severity_label(),
        finding.title
    )
    .unwrap();
    writeln!(output, "  - Confidence: {}", finding.confidence_label()).unwrap();
    if let Some(status) = status {
        writeln!(output, "  - Baseline: {status}").unwrap();
    }
    if let Some(location) = first_location(finding) {
        writeln!(output, "  - Location: `{location}`").unwrap();
    }
    for evidence in &finding.evidence {
        let location = if evidence.line_start > 0 {
            format!("{}:{}", evidence.path.display(), evidence.line_start)
        } else {
            evidence.path.display().to_string()
        };
        let snippet = evidence.snippet.trim();
        if snippet.is_empty() {
            writeln!(output, "  - Evidence: `{location}`").unwrap();
        } else {
            writeln!(
                output,
                "  - Evidence: `{location}` - {}",
                inline_snippet(snippet)
            )
            .unwrap();
        }
    }
    if !finding.description.is_empty() {
        writeln!(
            output,
            "  - Context: {}",
            first_sentence(&finding.description, 180)
        )
        .unwrap();
    }
    writeln!(
        output,
        "  - Recommendation: {}",
        first_sentence(finding.recommendation_or_default(), 220)
    )
    .unwrap();
    if let Some(url) = &finding.docs_url {
        writeln!(output, "  - Docs: {url}").unwrap();
    }
    output.push('\n');
}

fn render_react_native_section(output: &mut String, rn: &ReactNativeArchitectureProfile) {
    output.push_str("### React Native\n\n");

    let version = rn.react_native_version.as_deref().unwrap_or("unknown");
    writeln!(output, "- **Version:** {version}").unwrap();
    writeln!(output, "- **Project kind:** `{:?}`", rn.project_kind).unwrap();
    writeln!(
        output,
        "- **Package manager:** {}",
        rn.package_manager.as_deref().unwrap_or("unknown")
    )
    .unwrap();

    writeln!(
        output,
        "- **iOS:** {}",
        if rn.has_ios {
            "detected"
        } else {
            "not detected"
        }
    )
    .unwrap();
    writeln!(
        output,
        "- **Android:** {}",
        if rn.has_android {
            "detected"
        } else {
            "not detected"
        }
    )
    .unwrap();
    writeln!(
        output,
        "- **Android New Architecture:** {}",
        tristate_label(rn.android_new_arch_enabled)
    )
    .unwrap();
    writeln!(
        output,
        "- **iOS New Architecture:** {}",
        tristate_label(rn.ios_new_arch_enabled)
    )
    .unwrap();
    writeln!(
        output,
        "- **Expo New Architecture:** {}",
        tristate_label(rn.expo_new_arch_enabled)
    )
    .unwrap();
    writeln!(
        output,
        "- **Hermes:** {}",
        tristate_label(rn.hermes_enabled)
    )
    .unwrap();
    writeln!(
        output,
        "- **Codegen config:** {}\n",
        if rn.has_codegen_config {
            "found"
        } else {
            "missing"
        }
    )
    .unwrap();
}

fn render_frameworks_section(output: &mut String, frameworks: &[DetectedFramework]) {
    if frameworks.is_empty() {
        return;
    }
    let labels: Vec<String> = frameworks.iter().map(|f| f.label()).collect();
    output.push_str("## Frameworks\n\n");
    writeln!(output, "{}\n", labels.join(" | ")).unwrap();
}

fn render_framework_projects_section(output: &mut String, projects: &[FrameworkProject]) {
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

fn render_workspace_risk_table(output: &mut String, findings: &[Finding]) {
    let table = workspace_package_counts(findings);

    if table.is_empty() {
        return;
    }

    output.push_str("## Workspace Risk Summary\n\n");
    output.push_str("| Package | Critical | High | Medium | Low | Info | Total |\n");
    output.push_str("| --- | ---: | ---: | ---: | ---: | ---: | ---: |\n");
    for (pkg, counts) in &table {
        let total: usize = counts.iter().sum();
        writeln!(
            output,
            "| {} | {} | {} | {} | {} | {} | {} |",
            escape_table_cell(pkg),
            counts[0],
            counts[1],
            counts[2],
            counts[3],
            counts[4],
            total
        )
        .unwrap();
    }
    output.push('\n');
}

struct IndexRow {
    category: String,
    rule_id: String,
    severity: Severity,
    count: usize,
    new_count: usize,
    existing_count: usize,
    first_location: Option<String>,
}

fn grouped_index_rows(
    findings: &[Finding],
    baseline: Option<&BaselineScanReport>,
) -> Vec<IndexRow> {
    let mut rows: BTreeMap<(String, String), IndexRow> = BTreeMap::new();

    for (index, finding) in findings.iter().enumerate() {
        let key = (
            finding.category.label().to_string(),
            finding.rule_id.clone(),
        );
        let row = rows.entry(key).or_insert_with(|| IndexRow {
            category: finding.category.label().to_string(),
            rule_id: finding.rule_id.clone(),
            severity: finding.severity,
            count: 0,
            new_count: 0,
            existing_count: 0,
            first_location: first_location(finding),
        });
        row.count += 1;
        row.severity = row.severity.max(finding.severity);
        if row.first_location.is_none() {
            row.first_location = first_location(finding);
        }
        if let Some(report) = baseline {
            match report.finding_status(index).lowercase_label() {
                "new" => row.new_count += 1,
                "existing" => row.existing_count += 1,
                _ => {}
            }
        }
    }

    let mut rows = rows.into_values().collect::<Vec<_>>();
    rows.sort_by(|left, right| {
        category_label_rank(&left.category)
            .cmp(&category_label_rank(&right.category))
            .then_with(|| right.severity.cmp(&left.severity))
            .then_with(|| right.count.cmp(&left.count))
            .then_with(|| left.rule_id.cmp(&right.rule_id))
    });
    rows
}

fn inline_snippet(snippet: &str) -> String {
    snippet.replace('`', "'")
}
