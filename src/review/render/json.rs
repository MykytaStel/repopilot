use crate::baseline::gate::CiGateResult;
use crate::report::schema::ReviewJsonReport;
use crate::review::ReviewSignalGateResult;
use crate::review::model::ReviewReport;

pub fn render_json(
    report: &ReviewReport,
    ci_gate: Option<&CiGateResult>,
) -> Result<String, serde_json::Error> {
    render_json_with_gates(report, ci_gate, None)
}

pub fn render_json_with_gates(
    report: &ReviewReport,
    ci_gate: Option<&CiGateResult>,
    review_gate: Option<&ReviewSignalGateResult>,
) -> Result<String, serde_json::Error> {
    serde_json::to_string_pretty(&ReviewJsonReport::from_report(report, ci_gate, review_gate))
}
