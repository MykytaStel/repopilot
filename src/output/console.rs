use crate::baseline::diff::{BaselineScanReport, BaselineStatus};
use crate::baseline::gate::CiGateResult;
use crate::findings::types::{Finding, Severity};
use crate::frameworks::DetectedFramework;
use crate::output::color;
use crate::scan::types::ScanSummary;

pub fn render(summary: &ScanSummary) -> String {
    let mut output = String::new();

    output.push_str("RepoPilot Scan\n");
    output.push_str(&format!("Path: {}\n\n", summary.root_path.display()));

    output.push_str(&format!("Files analyzed: {}\n", summary.files_count));
    output.push_str(&format!(
        "Directories analyzed: {}\n",
        summary.directories_count
    ));
    output.push_str(&format!("Lines of code: {}\n\n", summary.lines_of_code));
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
    output.push_str(&format!("Lines of code: {}\n\n", summary.lines_of_code));
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

    for finding in findings {
        let label = color::severity_label(finding.severity_label());
        output.push_str(&format!(
            "  [{}] {} \u{2014} {}\n",
            label, finding.rule_id, finding.title
        ));

        for evidence in &finding.evidence {
            output.push_str(&format!(
                "    Evidence: {}:{} \u{2014} {}\n",
                evidence.path.display(),
                evidence.line_start,
                evidence.snippet.trim()
            ));
        }
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
    }
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
