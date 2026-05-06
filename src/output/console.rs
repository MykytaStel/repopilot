use crate::baseline::diff::{BaselineScanReport, BaselineStatus};
use crate::baseline::gate::CiGateResult;
use crate::findings::types::Finding;
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

    output.push_str("\nFindings:\n");

    if summary.findings.is_empty() {
        output.push_str("  No findings found\n");
    } else {
        for finding in &summary.findings {
            output.push_str(&format!(
                "  [{}] {} — {}\n",
                finding.severity_label(),
                finding.rule_id,
                finding.title
            ));

            for evidence in &finding.evidence {
                output.push_str(&format!(
                    "    Evidence: {}:{} — {}\n",
                    evidence.path.display(),
                    evidence.line_start,
                    evidence.snippet.trim()
                ));
            }
        }
    }

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

    output.push_str("\nFindings:\n");

    if summary.findings.is_empty() {
        output.push_str("  No findings found\n");
        return output;
    }

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

fn render_findings_group(output: &mut String, label: &str, findings: &[&Finding]) {
    output.push_str(&format!("  {label}: {}\n", findings.len()));

    for finding in findings {
        output.push_str(&format!(
            "    [{}] {} — {}\n",
            finding.severity_label(),
            finding.rule_id,
            finding.title
        ));

        for evidence in &finding.evidence {
            output.push_str(&format!(
                "      Evidence: {}:{} — {}\n",
                evidence.path.display(),
                evidence.line_start,
                evidence.snippet.trim()
            ));
        }
    }
}
