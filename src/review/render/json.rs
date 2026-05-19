use crate::baseline::gate::CiGateResult;
use crate::report::schema::ReviewJsonReport;
use crate::review::model::ReviewReport;

pub fn render_json(
    report: &ReviewReport,
    ci_gate: Option<&CiGateResult>,
) -> Result<String, serde_json::Error> {
    serde_json::to_string_pretty(&ReviewJsonReport::from_report(report, ci_gate))
}
