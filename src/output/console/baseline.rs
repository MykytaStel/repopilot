use crate::baseline::diff::BaselineScanReport;
use crate::baseline::gate::CiGateResult;
use std::fmt::Write;

pub(crate) fn render_baseline_block(
    output: &mut String,
    report: &BaselineScanReport,
    ci_gate: Option<&CiGateResult>,
) {
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
}
