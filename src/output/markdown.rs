use crate::baseline::diff::BaselineScanReport;
use crate::baseline::gate::CiGateResult;
use crate::findings::types::{Finding, FindingCategory, Severity};
use crate::frameworks::DetectedFramework;
use crate::frameworks::FrameworkProject;
use crate::frameworks::ReactNativeArchitectureProfile;
use crate::output::render_helpers::escape_table_cell;
use crate::output::report_stats::{
    ReportStats, TOOL_VERSION, build_report_stats, category_order, findings_for_category,
    findings_for_rule, first_location, rule_ids_for_findings, severity_order,
};
use crate::scan::types::ScanSummary;
use std::collections::BTreeMap;

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
        Some(path) => output.push_str(&format!("- **Baseline:** `{}`\n", path.display())),
        None => output.push_str("- **Baseline:** none (all findings treated as new)\n"),
    }
    output.push_str(&format!("- **New findings:** {}\n", report.new_count()));
    output.push_str(&format!(
        "- **Existing findings:** {}\n",
        report.existing_count()
    ));
    if let Some(ci_gate) = ci_gate {
        let status = if ci_gate.passed() { "passed" } else { "failed" };
        output.push_str(&format!(
            "- **CI gate:** {status} (`{}`)\n",
            ci_gate.label()
        ));
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
    output.push_str(&format!("- **RepoPilot version:** {TOOL_VERSION}\n"));
    output.push_str(&format!("- **Path:** `{}`\n", summary.root_path.display()));
    output.push_str(&format!("- **Risk:** {}\n", stats.risk_label));
    output.push_str(&format!("- **Health score:** {}/100\n", stats.health_score));
    output.push_str(&format!(
        "- **Findings:** {} ({:.1}/kloc)\n",
        stats.total_findings, stats.finding_density
    ));
    output.push_str(&format!("- **Files analyzed:** {}\n", summary.files_count));
    output.push_str(&format!(
        "- **Directories analyzed:** {}\n",
        summary.directories_count
    ));
    output.push_str(&format!("- **Lines of code:** {}\n", summary.lines_of_code));
    if summary.scan_duration_us > 0 {
        output.push_str(&format!(
            "- **Scan time:** {:.2}s\n",
            summary.scan_duration_us as f64 / 1_000_000.0
        ));
    }
    if summary.skipped_files_count > 0 {
        output.push_str(&format!(
            "- **Files skipped:** {} ({} bytes)\n",
            summary.skipped_files_count, summary.skipped_bytes
        ));
    }
    output.push('\n');
}

fn render_risk_summary(output: &mut String, summary: &ScanSummary, stats: &ReportStats) {
    output.push_str("## Risk Summary\n\n");

    if summary.findings.is_empty() {
        output.push_str("No findings found.\n\n");
        return;
    }

    output.push_str(&format!(
        "- **Severity:** {}\n",
        severity_counts_text(stats)
    ));
    output.push_str(&format!(
        "- **Categories:** {}\n",
        named_counts_text(&stats.category_counts)
    ));
    if !stats.top_paths.is_empty() {
        output.push_str(&format!(
            "- **Top paths:** {}\n",
            named_counts_text(&stats.top_paths)
        ));
    }
    if !stats.top_packages.is_empty() {
        output.push_str(&format!(
            "- **Top packages:** {}\n",
            named_counts_text(&stats.top_packages)
        ));
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
        output.push_str(&format!(
            "| `{}` | {} | {} |\n",
            escape_table_cell(&rule.label),
            rule.count,
            severity
        ));
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
        output.push_str(&format!(
            "| {} | {} |\n",
            escape_table_cell(&language.name),
            language.files_count
        ));
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
            output.push_str(&format!(
                "| {} | `{}` | {} | {} | {} | {} | {} |\n",
                escape_table_cell(&row.category),
                escape_table_cell(&row.rule_id),
                row.severity.label(),
                row.count,
                row.new_count,
                row.existing_count,
                escape_table_cell(&row.first_location.unwrap_or_else(|| "n/a".to_string()))
            ));
        }
    } else {
        output.push_str("| Category | Rule | Max severity | Count | First location |\n");
        output.push_str("| --- | --- | --- | ---: | --- |\n");
        for row in rows {
            output.push_str(&format!(
                "| {} | `{}` | {} | {} | {} |\n",
                escape_table_cell(&row.category),
                escape_table_cell(&row.rule_id),
                row.severity.label(),
                row.count,
                escape_table_cell(&row.first_location.unwrap_or_else(|| "n/a".to_string()))
            ));
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

        output.push_str(&format!("### {}\n\n", category_title(&category)));
        let rules = rule_ids_for_findings(&category_findings);
        for rule_id in rules {
            let rule_findings = findings_for_rule(&category_findings, &rule_id);
            output.push_str(&format!("#### `{rule_id}` ({})\n\n", rule_findings.len()));

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
    output.push_str(&format!(
        "- **[{}] {}**\n",
        finding.severity_label(),
        finding.title
    ));
    if let Some(status) = status {
        output.push_str(&format!("  - Baseline: {status}\n"));
    }
    if let Some(location) = first_location(finding) {
        output.push_str(&format!("  - Location: `{location}`\n"));
    }
    for evidence in &finding.evidence {
        let location = if evidence.line_start > 0 {
            format!("{}:{}", evidence.path.display(), evidence.line_start)
        } else {
            evidence.path.display().to_string()
        };
        let snippet = evidence.snippet.trim();
        if snippet.is_empty() {
            output.push_str(&format!("  - Evidence: `{location}`\n"));
        } else {
            output.push_str(&format!(
                "  - Evidence: `{location}` - {}\n",
                inline_snippet(snippet)
            ));
        }
    }
    if !finding.description.is_empty() {
        output.push_str(&format!(
            "  - Context: {}\n",
            first_sentence(&finding.description, 180)
        ));
    }
    if let Some(url) = &finding.docs_url {
        output.push_str(&format!("  - Docs: {url}\n"));
    }
    output.push('\n');
}

fn render_react_native_section(output: &mut String, rn: &ReactNativeArchitectureProfile) {
    output.push_str("### React Native\n\n");

    let version = rn.react_native_version.as_deref().unwrap_or("unknown");
    output.push_str(&format!("- **Version:** {version}\n"));
    output.push_str(&format!("- **Project kind:** `{:?}`\n", rn.project_kind));
    output.push_str(&format!(
        "- **Package manager:** {}\n",
        rn.package_manager.as_deref().unwrap_or("unknown")
    ));

    output.push_str(&format!(
        "- **iOS:** {}\n",
        if rn.has_ios {
            "detected"
        } else {
            "not detected"
        }
    ));
    output.push_str(&format!(
        "- **Android:** {}\n",
        if rn.has_android {
            "detected"
        } else {
            "not detected"
        }
    ));
    output.push_str(&format!(
        "- **Android New Architecture:** {}\n",
        format_tristate(rn.android_new_arch_enabled)
    ));
    output.push_str(&format!(
        "- **iOS New Architecture:** {}\n",
        format_tristate(rn.ios_new_arch_enabled)
    ));
    output.push_str(&format!(
        "- **Expo New Architecture:** {}\n",
        format_tristate(rn.expo_new_arch_enabled)
    ));
    output.push_str(&format!(
        "- **Hermes:** {}\n",
        format_tristate(rn.hermes_enabled)
    ));
    output.push_str(&format!(
        "- **Codegen config:** {}\n\n",
        if rn.has_codegen_config {
            "found"
        } else {
            "missing"
        }
    ));
}

fn format_tristate(value: Option<bool>) -> &'static str {
    match value {
        Some(true) => "enabled",
        Some(false) => "disabled",
        None => "unknown",
    }
}

fn render_frameworks_section(output: &mut String, frameworks: &[DetectedFramework]) {
    if frameworks.is_empty() {
        return;
    }
    let labels: Vec<String> = frameworks.iter().map(|f| f.label()).collect();
    output.push_str("## Frameworks\n\n");
    output.push_str(&format!("{}\n\n", labels.join(" | ")));
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
        output.push_str(&format!(
            "| `{}` | {} |\n",
            project.path.display(),
            escape_table_cell(&labels.join(", "))
        ));
    }
    output.push('\n');
}

fn render_workspace_risk_table(output: &mut String, findings: &[Finding]) {
    let has_workspace = findings.iter().any(|f| f.workspace_package.is_some());
    if !has_workspace {
        return;
    }

    let mut table: BTreeMap<&str, [usize; 5]> = BTreeMap::new();
    for f in findings {
        if let Some(pkg) = f.workspace_package.as_deref() {
            let counts = table.entry(pkg).or_insert([0; 5]);
            counts[crate::output::report_stats::severity_index(f.severity)] += 1;
        }
    }

    if table.is_empty() {
        return;
    }

    output.push_str("## Workspace Risk Summary\n\n");
    output.push_str("| Package | Critical | High | Medium | Low | Info | Total |\n");
    output.push_str("| --- | ---: | ---: | ---: | ---: | ---: | ---: |\n");
    for (pkg, counts) in &table {
        let total: usize = counts.iter().sum();
        output.push_str(&format!(
            "| {} | {} | {} | {} | {} | {} | {} |\n",
            escape_table_cell(pkg),
            counts[0],
            counts[1],
            counts[2],
            counts[3],
            counts[4],
            total
        ));
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

fn severity_counts_text(stats: &ReportStats) -> String {
    let parts = severity_order()
        .iter()
        .filter_map(|severity| {
            let count = stats.severity_count(*severity);
            (count > 0).then(|| format!("{} {}", count, severity.lowercase_label()))
        })
        .collect::<Vec<_>>();

    if parts.is_empty() {
        "none".to_string()
    } else {
        parts.join(", ")
    }
}

fn named_counts_text(counts: &[crate::output::report_stats::NamedCount]) -> String {
    if counts.is_empty() {
        return "none".to_string();
    }

    counts
        .iter()
        .map(|count| format!("{} ({})", count.label, count.count))
        .collect::<Vec<_>>()
        .join(", ")
}

fn category_title(category: &FindingCategory) -> &'static str {
    match category {
        FindingCategory::Security => "Security",
        FindingCategory::Architecture => "Architecture",
        FindingCategory::Framework => "Framework",
        FindingCategory::CodeQuality => "Code Quality",
        FindingCategory::Testing => "Testing",
    }
}

fn category_label_rank(label: &str) -> usize {
    match label {
        "security" => 0,
        "architecture" => 1,
        "framework" => 2,
        "code-quality" => 3,
        "testing" => 4,
        _ => usize::MAX,
    }
}

fn first_sentence(text: &str, max_len: usize) -> String {
    let sentence = text.split(". ").next().unwrap_or(text);
    if sentence.len() <= max_len {
        sentence.to_string()
    } else {
        format!("{}...", &sentence[..max_len])
    }
}

fn inline_snippet(snippet: &str) -> String {
    snippet.replace('`', "'")
}
