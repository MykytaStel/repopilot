use crate::baseline::diff::BaselineScanReport;
use crate::baseline::gate::CiGateResult;
use crate::findings::types::Finding;
use crate::frameworks::DetectedFramework;
use crate::frameworks::FrameworkProject;
use crate::frameworks::ReactNativeArchitectureProfile;
use crate::output::color;
use crate::output::report_stats::{
    ReportStats, TOOL_VERSION, build_report_stats, category_order, findings_for_category,
    findings_for_rule, first_location, rule_ids_for_findings, severity_index,
};
use crate::output::report_text::{
    category_title, console_severity_counts_text, first_sentence, named_counts_text, tristate_label,
};
use crate::scan::types::ScanSummary;
use std::collections::BTreeMap;

pub fn render(summary: &ScanSummary) -> String {
    let stats = build_report_stats(summary);
    let mut output = String::new();

    render_header(&mut output, summary, &stats);
    render_risk_summary(&mut output, &stats);
    render_top_rules(&mut output, &stats);
    render_languages_section(&mut output, summary);
    render_frameworks_section(&mut output, &summary.detected_frameworks);
    render_framework_projects_section(&mut output, &summary.framework_projects);
    if let Some(rn) = &summary.react_native {
        render_react_native_section(&mut output, rn);
    }
    workspace_risk_table(&mut output, &summary.findings);
    render_grouped_findings(&mut output, &summary.findings, |_| None);

    output
}

pub fn render_with_baseline(report: &BaselineScanReport, ci_gate: Option<&CiGateResult>) -> String {
    let summary = &report.summary;
    let stats = build_report_stats(summary);
    let mut output = String::new();

    render_header(&mut output, summary, &stats);
    match &report.baseline_path {
        Some(path) => output.push_str(&format!("Baseline: {}\n", path.display())),
        None => output.push_str("Baseline: none (all findings treated as new)\n"),
    }
    output.push_str(&format!("New findings: {}\n", report.new_count()));
    output.push_str(&format!("Existing findings: {}\n", report.existing_count()));
    if let Some(ci_gate) = ci_gate {
        let status = if ci_gate.passed() { "passed" } else { "failed" };
        output.push_str(&format!("CI gate: {status} ({})\n", ci_gate.label()));
    }
    output.push('\n');

    render_risk_summary(&mut output, &stats);
    render_top_rules(&mut output, &stats);
    render_languages_section(&mut output, summary);
    render_frameworks_section(&mut output, &summary.detected_frameworks);
    render_framework_projects_section(&mut output, &summary.framework_projects);
    if let Some(rn) = &summary.react_native {
        render_react_native_section(&mut output, rn);
    }
    workspace_risk_table(&mut output, &summary.findings);
    render_grouped_findings(&mut output, &summary.findings, |index| {
        Some(report.finding_status(index).lowercase_label())
    });

    output
}

fn render_header(output: &mut String, summary: &ScanSummary, stats: &ReportStats) {
    output.push_str("RepoPilot Scan\n");
    output.push_str(&format!("Version: {TOOL_VERSION}\n"));
    output.push_str(&format!("Path: {}\n", summary.root_path.display()));
    output.push_str(&format!(
        "Risk: {} | Health score: {}/100 {}\n",
        stats.risk_label,
        stats.health_score,
        health_score_bar(stats.health_score)
    ));
    output.push_str(&format!(
        "Findings: {} ({:.1}/kloc)\n",
        stats.total_findings, stats.finding_density
    ));
    output.push_str(&format!(
        "Directories analyzed: {} | Lines of code: {}\n",
        summary.directories_count, summary.lines_of_code
    ));
    render_scan_input(output, summary);
    if summary.scan_duration_us > 0 {
        output.push_str(&format!(
            "Scan time: {:.2}s\n",
            summary.scan_duration_us as f64 / 1_000_000.0
        ));
    }
    output.push('\n');
}

fn render_scan_input(output: &mut String, summary: &ScanSummary) {
    output.push_str("Scan input:\n");

    if let Some(path) = &summary.repopilotignore_path {
        output.push_str(&format!(" .repopilotignore: {}\n", path.display()));
    }

    if summary.files_skipped_repopilotignore > 0 {
        output.push_str(&format!(
            " Files skipped (.repopilotignore): {:>7}\n",
            summary.files_skipped_repopilotignore
        ));
    }

    output.push_str(&format!(
        " Files discovered: {:>7}\n",
        summary.files_discovered
    ));

    if summary.files_skipped_by_limit > 0 {
        output.push_str(&format!(
            " Files skipped (limit): {:>7}\n",
            summary.files_skipped_by_limit
        ));
    }

    output.push_str(&format!(" Files analyzed: {:>7}\n", summary.files_count));

    if summary.skipped_files_count > 0 {
        output.push_str(&format!(
            " Large files skipped: {:>7}\n",
            summary.skipped_files_count
        ));
    }

    if summary.binary_files_skipped > 0 {
        output.push_str(&format!(
            " Binary files skipped: {:>7}\n",
            summary.binary_files_skipped
        ));
    }

    if summary.files_skipped_low_signal > 0 {
        output.push_str(&format!(
            " Low-signal files skipped:{:>7}\n",
            summary.files_skipped_low_signal
        ));
    }
}

fn render_risk_summary(output: &mut String, stats: &ReportStats) {
    output.push_str("Risk Summary:\n");
    output.push_str(&format!(
        "  Severity: {}\n",
        console_severity_counts_text(stats)
    ));
    output.push_str(&format!(
        "  Categories: {}\n",
        named_counts_text(&stats.category_counts)
    ));
    if !stats.top_paths.is_empty() {
        output.push_str(&format!(
            "  Top paths: {}\n",
            named_counts_text(&stats.top_paths)
        ));
    }
    if !stats.top_packages.is_empty() {
        output.push_str(&format!(
            "  Top packages: {}\n",
            named_counts_text(&stats.top_packages)
        ));
    }
    output.push('\n');
}

fn render_top_rules(output: &mut String, stats: &ReportStats) {
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
        output.push_str(&format!(
            "  {:>4}  [{}] {}\n",
            rule.count, severity, rule.label
        ));
    }
    output.push('\n');
}

fn render_languages_section(output: &mut String, summary: &ScanSummary) {
    output.push_str("Languages:\n");

    if summary.languages.is_empty() {
        output.push_str("  No languages detected\n\n");
        return;
    }

    for language in &summary.languages {
        output.push_str(&format!(
            "  {}: {} files\n",
            language.name, language.files_count
        ));
    }
    output.push('\n');
}

fn render_grouped_findings<F>(output: &mut String, findings: &[Finding], status_for: F)
where
    F: Fn(usize) -> Option<&'static str>,
{
    output.push_str("Findings:\n");

    if findings.is_empty() {
        output.push_str("  none\n");
        return;
    }

    for category in category_order() {
        let category_findings = findings_for_category(findings, &category);
        if category_findings.is_empty() {
            continue;
        }

        output.push_str(&format!("  {}:\n", category_title(&category)));
        let rules = rule_ids_for_findings(&category_findings);
        for rule_id in rules {
            let rule_findings = findings_for_rule(&category_findings, &rule_id);
            output.push_str(&format!("    {} ({})\n", rule_id, rule_findings.len()));
            for finding in rule_findings {
                let index = findings
                    .iter()
                    .position(|candidate| std::ptr::eq(candidate, finding))
                    .unwrap_or(0);
                render_finding(output, finding, status_for(index));
            }
        }
        output.push('\n');
    }
}

fn render_finding(output: &mut String, finding: &Finding, status: Option<&str>) {
    let severity = color::severity_label(finding.severity_label());
    output.push_str(&format!("      [{}] {}\n", severity, finding.title));
    if let Some(status) = status {
        output.push_str(&format!("        Baseline: {status}\n"));
    }
    if let Some(location) = first_location(finding) {
        output.push_str(&format!("        Location: {location}\n"));
    }
    for evidence in &finding.evidence {
        let location = if evidence.line_start > 0 {
            format!("{}:{}", evidence.path.display(), evidence.line_start)
        } else {
            evidence.path.display().to_string()
        };
        let snippet = evidence.snippet.trim();
        if snippet.is_empty() {
            output.push_str(&format!("        Evidence: {location}\n"));
        } else {
            output.push_str(&format!("        Evidence: {location} - {snippet}\n"));
        }
    }
    if !finding.description.is_empty() {
        output.push_str(&format!(
            "        {}\n",
            color::dim(&first_sentence(&finding.description, 120))
        ));
    }
    if let Some(url) = &finding.docs_url {
        output.push_str(&format!("        Docs: {url}\n"));
    }
}

fn workspace_risk_table(output: &mut String, findings: &[Finding]) {
    let has_workspace = findings.iter().any(|f| f.workspace_package.is_some());
    if !has_workspace {
        return;
    }

    let mut table: BTreeMap<&str, [usize; 5]> = BTreeMap::new();
    for f in findings {
        if let Some(pkg) = f.workspace_package.as_deref() {
            let counts = table.entry(pkg).or_insert([0; 5]);
            counts[severity_index(f.severity)] += 1;
        }
    }

    if table.is_empty() {
        return;
    }

    let name_width = table.keys().map(|k| k.len()).max().unwrap_or(7).max(7);

    output.push_str("Workspace Risk Summary:\n");
    output.push_str(&format!(
        "  {:<width$}  {:>5}  {:>5}  {:>5}  {:>5}  {:>5}  {:>5}\n",
        "Package",
        "Crit",
        "High",
        "Med",
        "Low",
        "Info",
        "Total",
        width = name_width
    ));
    output.push_str(&format!(
        "  {:-<width$}  -----  -----  -----  -----  -----  -----\n",
        "",
        width = name_width
    ));
    for (pkg, counts) in &table {
        let total: usize = counts.iter().sum();
        output.push_str(&format!(
            "  {:<width$}  {:>5}  {:>5}  {:>5}  {:>5}  {:>5}  {:>5}\n",
            pkg,
            counts[0],
            counts[1],
            counts[2],
            counts[3],
            counts[4],
            total,
            width = name_width
        ));
    }
    output.push('\n');
}

fn render_frameworks_section(output: &mut String, frameworks: &[DetectedFramework]) {
    if frameworks.is_empty() {
        return;
    }
    let labels: Vec<String> = frameworks.iter().map(|f| f.label()).collect();
    output.push_str(&format!("Frameworks: {}\n\n", labels.join(" | ")));
}

fn render_framework_projects_section(output: &mut String, projects: &[FrameworkProject]) {
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
        output.push_str(&format!(
            "  {}: {}\n",
            project.path.display(),
            labels.join(" | ")
        ));
    }
    output.push('\n');
}

fn render_react_native_section(output: &mut String, rn: &ReactNativeArchitectureProfile) {
    let version = rn.react_native_version.as_deref().unwrap_or("unknown");
    let ios = if rn.has_ios { "yes" } else { "no" };
    let android = if rn.has_android { "yes" } else { "no" };
    let new_arch_android = tristate_label(rn.android_new_arch_enabled);
    let new_arch_ios = tristate_label(rn.ios_new_arch_enabled);
    let new_arch_expo = tristate_label(rn.expo_new_arch_enabled);
    let hermes = tristate_label(rn.hermes_enabled);
    let codegen = if rn.has_codegen_config { "yes" } else { "no" };
    let package_manager = rn.package_manager.as_deref().unwrap_or("unknown");

    output.push_str("React Native:\n");
    output.push_str(&format!(
        "  Version: {version}  Kind: {:?}  Package manager: {package_manager}\n",
        rn.project_kind
    ));
    output.push_str(&format!(
        "  iOS: {ios}  Android: {android}  Expo config: {}\n",
        if rn.has_expo_config { "yes" } else { "no" }
    ));
    output.push_str(&format!(
        "  New Arch (Android): {new_arch_android}  New Arch (iOS): {new_arch_ios}  New Arch (Expo): {new_arch_expo}\n"
    ));
    output.push_str(&format!("  Hermes: {hermes}  Codegen: {codegen}\n\n"));
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
