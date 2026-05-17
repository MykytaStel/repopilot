use crate::baseline::diff::BaselineScanReport;
use crate::baseline::gate::CiGateResult;
use crate::findings::types::Finding;
use crate::frameworks::DetectedFramework;
use crate::frameworks::FrameworkProject;
use crate::frameworks::ReactNativeArchitectureProfile;
use crate::output::color;
use crate::output::finding_helpers::{clusters_by_rule_scope, example_locations};
use crate::output::render_helpers::workspace_package_rows;
use crate::output::report_stats::{
    ReportStats, TOOL_VERSION, build_report_stats, category_order, findings_for_category,
    findings_for_rule, first_location, rule_ids_for_findings,
};
use crate::output::report_text::{
    category_title, console_severity_counts_text, first_sentence, named_counts_text, tristate_label,
};
use crate::scan::types::ScanSummary;
use std::fmt::Write;

pub fn render(summary: &ScanSummary) -> String {
    let stats = build_report_stats(summary);
    let mut output = String::new();

    render_header(&mut output, summary, &stats);
    render_risk_summary(&mut output, &stats);
    render_top_risk_clusters(&mut output, &summary.findings);
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
        Some(path) => writeln!(output, "Baseline: {}", path.display()).unwrap(),
        None => output.push_str("Baseline: none (all findings treated as new)\n"),
    }
    writeln!(output, "New findings: {}", report.new_count()).unwrap();
    writeln!(output, "Existing findings: {}", report.existing_count()).unwrap();
    if let Some(ci_gate) = ci_gate {
        let status = if ci_gate.passed() { "passed" } else { "failed" };
        writeln!(output, "CI gate: {status} ({})", ci_gate.label()).unwrap();
    }
    output.push('\n');

    render_risk_summary(&mut output, &stats);
    render_top_risk_clusters(&mut output, &summary.findings);
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
        "Findings: {} ({:.1}/kloc)",
        stats.total_findings, stats.finding_density
    )
    .unwrap();
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

fn render_risk_summary(output: &mut String, stats: &ReportStats) {
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
        stats.priority_count(crate::risk::RiskPriority::P0),
        stats.priority_count(crate::risk::RiskPriority::P1),
        stats.priority_count(crate::risk::RiskPriority::P2),
        stats.priority_count(crate::risk::RiskPriority::P3),
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
        writeln!(output, "  {:>4}  [{}] {}", rule.count, severity, rule.label).unwrap();
    }
    output.push('\n');
}

fn render_top_risk_clusters(output: &mut String, findings: &[Finding]) {
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

fn priority_rank(priority: crate::risk::RiskPriority) -> u8 {
    match priority {
        crate::risk::RiskPriority::P0 => 0,
        crate::risk::RiskPriority::P1 => 1,
        crate::risk::RiskPriority::P2 => 2,
        crate::risk::RiskPriority::P3 => 3,
    }
}

fn render_languages_section(output: &mut String, summary: &ScanSummary) {
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

        writeln!(output, "  {}:", category_title(&category)).unwrap();
        let rules = rule_ids_for_findings(&category_findings);
        for rule_id in rules {
            let rule_findings = findings_for_rule(&category_findings, &rule_id);
            writeln!(output, "    {} ({})", rule_id, rule_findings.len()).unwrap();
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
    writeln!(output, "      [{}] {}", severity, finding.title).unwrap();
    writeln!(output, "        Confidence: {}", finding.confidence_label()).unwrap();
    writeln!(
        output,
        "        Priority: {} (risk {}/100)",
        finding.risk.priority.label(),
        finding.risk.score
    )
    .unwrap();
    if let Some(reasons) = risk_reason_text(finding) {
        writeln!(output, "        Risk signals: {reasons}").unwrap();
    }
    if let Some(status) = status {
        writeln!(output, "        Baseline: {status}").unwrap();
    }
    if let Some(location) = first_location(finding) {
        writeln!(output, "        Location: {location}").unwrap();
    }
    for evidence in &finding.evidence {
        let location = if evidence.line_start > 0 {
            format!("{}:{}", evidence.path.display(), evidence.line_start)
        } else {
            evidence.path.display().to_string()
        };
        let snippet = evidence.snippet.trim();
        if snippet.is_empty() {
            writeln!(output, "        Evidence: {location}").unwrap();
        } else {
            writeln!(output, "        Evidence: {location} - {snippet}").unwrap();
        }
    }
    if !finding.description.is_empty() {
        writeln!(
            output,
            "        {}",
            color::dim(&first_sentence(&finding.description, 120))
        )
        .unwrap();
    }
    writeln!(
        output,
        "        Recommendation: {}",
        first_sentence(finding.recommendation_or_default(), 180)
    )
    .unwrap();
    if let Some(url) = &finding.docs_url {
        writeln!(output, "        Docs: {url}").unwrap();
    }
}

fn risk_reason_text(finding: &Finding) -> Option<String> {
    let reasons = finding
        .risk
        .signals
        .iter()
        .filter(|signal| !signal.id.starts_with("severity."))
        .take(3)
        .map(|signal| format!("{} ({:+})", signal.label, signal.weight))
        .collect::<Vec<_>>();

    (!reasons.is_empty()).then(|| reasons.join(", "))
}

fn workspace_risk_table(output: &mut String, findings: &[Finding]) {
    let rows = workspace_package_rows(findings);

    if rows.is_empty() {
        return;
    }

    let name_width = rows
        .iter()
        .map(|row| row.package.len())
        .max()
        .unwrap_or(7)
        .max(7);

    output.push_str("Workspace Risk Summary:\n");
    writeln!(
        output,
        "  {:<width$}  {:>5}  {:>5}  {:>5}  {:>5}  {:>5}  {:>5}",
        "Package",
        "Crit",
        "High",
        "Med",
        "Low",
        "Info",
        "Total",
        width = name_width
    )
    .unwrap();
    writeln!(
        output,
        "  {:-<width$}  -----  -----  -----  -----  -----  -----",
        "",
        width = name_width
    )
    .unwrap();
    for row in &rows {
        writeln!(
            output,
            "  {:<width$}  {:>5}  {:>5}  {:>5}  {:>5}  {:>5}  {:>5}",
            row.package.as_str(),
            row.counts[0],
            row.counts[1],
            row.counts[2],
            row.counts[3],
            row.counts[4],
            row.total,
            width = name_width
        )
        .unwrap();
    }
    output.push('\n');
}

fn render_frameworks_section(output: &mut String, frameworks: &[DetectedFramework]) {
    if frameworks.is_empty() {
        return;
    }
    let labels: Vec<String> = frameworks.iter().map(|f| f.label()).collect();
    writeln!(output, "Frameworks: {}\n", labels.join(" | ")).unwrap();
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
    writeln!(
        output,
        "  Version: {version}  Kind: {:?}  Package manager: {package_manager}",
        rn.project_kind
    )
    .unwrap();
    writeln!(
        output,
        "  iOS: {ios}  Android: {android}  Expo config: {}",
        if rn.has_expo_config { "yes" } else { "no" }
    )
    .unwrap();
    writeln!(
        output,
        "  New Arch (Android): {new_arch_android}  New Arch (iOS): {new_arch_ios}  New Arch (Expo): {new_arch_expo}"
    )
    .unwrap();
    writeln!(output, "  Hermes: {hermes}  Codegen: {codegen}\n").unwrap();
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
