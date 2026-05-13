use crate::baseline::gate::CiGateResult;
use crate::findings::types::Finding;
use crate::review::model::ReviewReport;
use crate::review::render::helpers::render_ranges_suffix;

pub fn render_console(report: &ReviewReport, ci_gate: Option<&CiGateResult>) -> String {
    let mut output = String::new();

    output.push_str("RepoPilot Review\n");
    output.push_str(&format!("Path: {}\n", report.summary.root_path.display()));
    output.push_str(&format!("Git root: {}\n", report.repo_root.display()));
    match &report.baseline_path {
        Some(path) => output.push_str(&format!("Baseline: {}\n", path.display())),
        None => output.push_str("Baseline: none (all findings treated as new)\n"),
    }
    output.push('\n');

    output.push_str(&format!("Changed files: {}\n", report.changed_files.len()));
    output.push_str(&format!("In-diff findings: {}\n", report.in_diff_count()));
    output.push_str(&format!(
        "Out-of-diff findings: {}\n",
        report.out_of_diff_count()
    ));
    output.push_str(&format!(
        "New in-diff findings: {}\n",
        report.new_in_diff_count()
    ));
    output.push_str(&format!(
        "Existing in-diff findings: {}\n",
        report.existing_in_diff_count()
    ));

    if let Some(ci_gate) = ci_gate {
        let status = if ci_gate.passed() { "passed" } else { "failed" };
        output.push_str(&format!("CI gate: {status} ({})\n", ci_gate.label()));
    }

    output.push_str("\nChanged files:\n");
    if report.changed_files.is_empty() {
        output.push_str("  No changed files found\n");
    } else {
        for file in &report.changed_files {
            output.push_str(&format!(
                "  {:?} {}{}\n",
                file.status,
                file.path.display(),
                render_ranges_suffix(file)
            ));
        }
    }

    render_blast_radius(&mut output, report);
    render_findings_group(&mut output, "In-diff findings", &report.in_diff_findings());
    render_findings_group(
        &mut output,
        "Out-of-diff findings",
        &report.out_of_diff_findings(),
    );

    output
}

fn render_blast_radius(output: &mut String, report: &ReviewReport) {
    if report.blast_radius.is_empty() {
        return;
    }

    output.push_str("\nBlast radius:\n");
    output.push_str("  The following files import changed files and may need extra review:\n");

    for path in &report.blast_radius {
        output.push_str(&format!("  - {}\n", path.display()));
    }
}

fn render_findings_group(output: &mut String, label: &str, findings: &[&Finding]) {
    output.push_str(&format!("\n{label}: {}\n", findings.len()));

    if findings.is_empty() {
        return;
    }

    for finding in findings {
        output.push_str(&format!(
            "  [{} confidence={}] {} - {}\n",
            finding.severity_label(),
            finding.confidence_label(),
            finding.rule_id,
            finding.title
        ));

        for evidence in &finding.evidence {
            output.push_str(&format!(
                "    Evidence: {}:{} - {}\n",
                evidence.path.display(),
                evidence.line_start,
                evidence.snippet.trim()
            ));
        }
    }
}
