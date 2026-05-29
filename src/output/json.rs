use crate::baseline::diff::BaselineScanReport;
use crate::baseline::gate::CiGateResult;
use crate::report::schema::{BaselineJsonReport, ScanJsonReport};
use crate::scan::types::ScanSummary;

pub use crate::report::schema::{REPOPILOT_VERSION, SCAN_REPORT_SCHEMA_VERSION};

pub fn render(summary: &ScanSummary) -> Result<String, serde_json::Error> {
    serde_json::to_string_pretty(&ScanJsonReport::from_summary(summary))
}

pub fn render_with_baseline(
    report: &BaselineScanReport,
    ci_gate: Option<&CiGateResult>,
) -> Result<String, serde_json::Error> {
    serde_json::to_string_pretty(&BaselineJsonReport::from_report(report, ci_gate))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::baseline::diff::{BaselineScanReport, FindingBaselineStatus};
    use crate::scan::types::{HiddenSuggestionSummary, ScanArtifacts, ScanMetadata, ScanMetrics};
    use serde_json::Value;
    use std::path::PathBuf;

    #[test]
    fn json_scan_report_includes_schema_and_tool_versions() {
        let summary = ScanSummary {
            metadata: ScanMetadata {
                root_path: PathBuf::from("."),
                ..Default::default()
            },
            metrics: ScanMetrics {
                files_analyzed: 1,
                ..Default::default()
            },
            artifacts: ScanArtifacts {
                hidden_suggestions: Vec::new(),
                ..Default::default()
            },
        };

        let rendered = render(&summary).expect("json render should succeed");
        let value: Value = serde_json::from_str(&rendered).expect("json should parse");

        assert_eq!(value["schema_version"], SCAN_REPORT_SCHEMA_VERSION);
        assert_eq!(value["repopilot_version"], REPOPILOT_VERSION);
        assert_eq!(value["report"]["kind"], "scan");
        assert_eq!(
            value["report"]["schema_version"],
            SCAN_REPORT_SCHEMA_VERSION
        );
        assert_eq!(value["files_analyzed"], 1);
        assert!(value.get("findings").is_some());
    }

    #[test]
    fn baseline_json_report_includes_schema_and_tool_versions() {
        let summary = ScanSummary {
            metadata: ScanMetadata {
                root_path: PathBuf::from("."),
                ..Default::default()
            },
            metrics: ScanMetrics {
                hidden_suggestions_count: 5,
                files_analyzed: 2,
                ..Default::default()
            },
            artifacts: ScanArtifacts {
                hidden_suggestions: vec![HiddenSuggestionSummary {
                    intent: "testing-gap".to_string(),
                    rule_id: "testing.source-without-test".to_string(),
                    category: "testing".to_string(),
                    reason: "testing gaps are hidden in the default profile".to_string(),
                    count: 5,
                }],
                ..Default::default()
            },
        };
        let report = BaselineScanReport {
            summary,
            baseline_path: Some(PathBuf::from(".repopilot/baseline.json")),
            findings: Vec::<FindingBaselineStatus>::new(),
        };

        let rendered =
            render_with_baseline(&report, None).expect("baseline json render should succeed");
        let value: Value = serde_json::from_str(&rendered).expect("json should parse");

        assert_eq!(value["schema_version"], SCAN_REPORT_SCHEMA_VERSION);
        assert_eq!(value["repopilot_version"], REPOPILOT_VERSION);
        assert_eq!(value["report"]["kind"], "baseline-scan");
        assert_eq!(value["files_analyzed"], 2);
        assert_eq!(value["hidden_suggestions_count"], 5);
        assert_eq!(
            value["hidden_suggestions"][0]["rule_id"],
            "testing.source-without-test"
        );
        assert!(value.get("baseline").is_some());
    }
}
