use crate::baseline::diff::{BaselineScanReport, BaselineStatus};
use crate::baseline::gate::CiGateResult;
use crate::findings::types::{Finding, Severity};
use crate::frameworks::DetectedFramework;
use crate::frameworks::FrameworkProject;
use crate::frameworks::ReactNativeArchitectureProfile;
use crate::output::color;
use crate::scan::types::ScanSummary;
use std::collections::BTreeMap;

pub fn render(summary: &ScanSummary) -> String {
    let mut output = String::new();

    output.push_str("RepoPilot Scan\n");
    output.push_str(&format!("Path: {}\n\n", summary.root_path.display()));

    output.push_str(&format!("Files analyzed: {}\n", summary.files_count));
    output.push_str(&format!(
        "Directories analyzed: {}\n",
        summary.directories_count
    ));
    output.push_str(&format!("Lines of code: {}\n", summary.lines_of_code));
    if summary.scan_duration_us > 0 {
        output.push_str(&format!(
            "Scan time: {:.2}s\n",
            summary.scan_duration_us as f64 / 1_000_000.0
        ));
    }
    output.push('\n');
    if summary.skipped_files_count > 0 {
        output.push_str(&format!(
            "Files skipped: {} ({} bytes)\n\n",
            summary.skipped_files_count, summary.skipped_bytes
        ));
    }

    output.push_str("Languages:\n");

    if summary.languages.is_empty() {
        output.push_str("  No languages detected\n");
    } else {
        for language in &summary.languages {
            output.push_str(&format!(
                "  {}: {} files\n",
                language.name, language.files_count
            ));
        }
    }

    output.push('\n');
    render_frameworks_section(&mut output, &summary.detected_frameworks);
    render_framework_projects_section(&mut output, &summary.framework_projects);
    if let Some(rn) = &summary.react_native {
        render_react_native_section(&mut output, rn);
    }
    workspace_risk_table(&mut output, &summary.findings);
    render_findings_section(&mut output, &summary.findings);

    output
}

pub fn render_with_baseline(report: &BaselineScanReport, ci_gate: Option<&CiGateResult>) -> String {
    let summary = &report.summary;
    let mut output = String::new();

    output.push_str("RepoPilot Scan\n");
    output.push_str(&format!("Path: {}\n", summary.root_path.display()));
    match &report.baseline_path {
        Some(path) => output.push_str(&format!("Baseline: {}\n", path.display())),
        None => output.push_str("Baseline: none (all findings treated as new)\n"),
    }
    output.push('\n');

    output.push_str(&format!("Files analyzed: {}\n", summary.files_count));
    output.push_str(&format!(
        "Directories analyzed: {}\n",
        summary.directories_count
    ));
    output.push_str(&format!("Lines of code: {}\n", summary.lines_of_code));
    if summary.scan_duration_us > 0 {
        output.push_str(&format!(
            "Scan time: {:.2}s\n",
            summary.scan_duration_us as f64 / 1_000_000.0
        ));
    }
    output.push('\n');
    if summary.skipped_files_count > 0 {
        output.push_str(&format!(
            "Files skipped: {} ({} bytes)\n\n",
            summary.skipped_files_count, summary.skipped_bytes
        ));
    }

    output.push_str(&format!("New findings: {}\n", report.new_count()));
    output.push_str(&format!("Existing findings: {}\n", report.existing_count()));

    if let Some(ci_gate) = ci_gate {
        let status = if ci_gate.passed() { "passed" } else { "failed" };
        output.push_str(&format!("CI gate: {status} ({})\n", ci_gate.label()));
    }

    output.push_str("\nLanguages:\n");

    if summary.languages.is_empty() {
        output.push_str("  No languages detected\n");
    } else {
        for language in &summary.languages {
            output.push_str(&format!(
                "  {}: {} files\n",
                language.name, language.files_count
            ));
        }
    }

    output.push('\n');
    render_frameworks_section(&mut output, &summary.detected_frameworks);
    render_framework_projects_section(&mut output, &summary.framework_projects);
    if let Some(rn) = &summary.react_native {
        render_react_native_section(&mut output, rn);
    }

    if summary.findings.is_empty() {
        output.push_str("Findings: none\n");
        return output;
    }

    render_severity_summary(&mut output, &summary.findings);

    output.push('\n');

    render_findings_group(
        &mut output,
        "New findings",
        &report.findings_with_status(BaselineStatus::New),
    );
    render_findings_group(
        &mut output,
        "Existing findings",
        &report.findings_with_status(BaselineStatus::Existing),
    );

    output
}

fn render_findings_section(output: &mut String, findings: &[Finding]) {
    if findings.is_empty() {
        output.push_str("Findings: none\n");
        return;
    }

    render_severity_summary(output, findings);
    output.push('\n');

    let has_workspace = findings.iter().any(|f| f.workspace_package.is_some());
    if has_workspace {
        let mut groups: Vec<(Option<&str>, Vec<&Finding>)> = Vec::new();
        for finding in findings {
            let key = finding.workspace_package.as_deref();
            if let Some(group) = groups.iter_mut().find(|(k, _)| *k == key) {
                group.1.push(finding);
            } else {
                groups.push((key, vec![finding]));
            }
        }
        for (pkg, group_findings) in &groups {
            let header = pkg.unwrap_or("(root)");
            output.push_str(&format!("  [{header}]\n"));
            render_findings_list(output, group_findings, "    ");
            output.push('\n');
        }
    } else {
        render_findings_list(output, &findings.iter().collect::<Vec<_>>(), "  ");
    }
}

fn render_findings_list(output: &mut String, findings: &[&Finding], indent: &str) {
    for finding in findings {
        let label = color::severity_label(finding.severity_label());
        output.push_str(&format!(
            "{indent}[{}] {} \u{2014} {}\n",
            label, finding.rule_id, finding.title
        ));
        for evidence in &finding.evidence {
            output.push_str(&format!(
                "{indent}  Evidence: {}:{} \u{2014} {}\n",
                evidence.path.display(),
                evidence.line_start,
                evidence.snippet.trim()
            ));
        }
        if !finding.description.is_empty() {
            let hint = first_sentence(&finding.description, 120);
            output.push_str(&format!("{indent}  {}\n", color::dim(&hint)));
        }
        if let Some(url) = &finding.docs_url {
            output.push_str(&format!("{indent}  Docs: {url}\n"));
        }
    }
}

fn first_sentence(text: &str, max_len: usize) -> String {
    let sentence = text.split(". ").next().unwrap_or(text);
    if sentence.len() <= max_len {
        sentence.to_string()
    } else {
        format!("{}…", &sentence[..max_len])
    }
}

fn render_findings_group(output: &mut String, label: &str, findings: &[&Finding]) {
    output.push_str(&format!("  {label}: {}\n", findings.len()));

    for finding in findings {
        let severity = color::severity_label(finding.severity_label());
        output.push_str(&format!(
            "    [{}] {} \u{2014} {}\n",
            severity, finding.rule_id, finding.title
        ));

        for evidence in &finding.evidence {
            output.push_str(&format!(
                "      Evidence: {}:{} \u{2014} {}\n",
                evidence.path.display(),
                evidence.line_start,
                evidence.snippet.trim()
            ));
        }
        if !finding.description.is_empty() {
            let hint = first_sentence(&finding.description, 120);
            output.push_str(&format!("      {}\n", color::dim(&hint)));
        }
        if let Some(url) = &finding.docs_url {
            output.push_str(&format!("      Docs: {url}\n"));
        }
    }
}

fn workspace_risk_table(output: &mut String, findings: &[Finding]) {
    let has_workspace = findings.iter().any(|f| f.workspace_package.is_some());
    if !has_workspace {
        return;
    }

    // Aggregate counts per package: [critical, high, medium, low, info]
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

    // Measure column widths
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

/// Renders a one-line severity tally: e.g. `Findings: 1 critical · 3 high · 5 medium`
fn render_severity_summary(output: &mut String, findings: &[Finding]) {
    // Single pass over findings to count each severity level
    let mut counts = [0usize; 5];
    for f in findings {
        counts[severity_index(f.severity)] += 1;
    }

    const LEVELS: [Severity; 5] = [
        Severity::Critical,
        Severity::High,
        Severity::Medium,
        Severity::Low,
        Severity::Info,
    ];

    let parts: Vec<String> = LEVELS
        .iter()
        .zip(counts.iter())
        .filter(|(_, n)| **n > 0)
        .map(|(sev, n)| color::severity_count(*sev, *n))
        .collect();

    output.push_str(&format!("Findings: {}\n", parts.join(" \u{00b7} ")));
}

fn severity_index(s: Severity) -> usize {
    match s {
        Severity::Critical => 0,
        Severity::High => 1,
        Severity::Medium => 2,
        Severity::Low => 3,
        Severity::Info => 4,
    }
}

fn render_frameworks_section(output: &mut String, frameworks: &[DetectedFramework]) {
    if frameworks.is_empty() {
        return;
    }
    let labels: Vec<String> = frameworks.iter().map(|f| f.label()).collect();
    output.push_str(&format!("Frameworks: {}\n\n", labels.join(" \u{00b7} ")));
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
            labels.join(" \u{00b7} ")
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

fn tristate_label(value: Option<bool>) -> &'static str {
    match value {
        Some(true) => "enabled",
        Some(false) => "disabled",
        None => "unknown",
    }
}
