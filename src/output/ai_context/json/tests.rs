use super::*;
use crate::findings::types::{Confidence, Evidence, FindingCategory};
use crate::scan::types::{ScanArtifacts, ScanMetadata, ScanSummary};
use std::path::PathBuf;

fn finding(index: usize, severity: Severity) -> Finding {
    Finding {
        id: format!("rule.example:src/file_{index}.rs:1"),
        rule_id: "rule.example".to_string(),
        title: format!("Example finding {index}"),
        description: "A reasonably long description so each finding carries real \
            weight in the budget accounting and truncation is exercised."
            .to_string(),
        recommendation: "Review and fix the example finding.".to_string(),
        category: FindingCategory::Security,
        severity,
        confidence: Confidence::High,
        evidence: vec![Evidence {
            path: PathBuf::from(format!("src/file_{index}.rs")),
            line_start: 1,
            line_end: Some(2),
            snippet: "let secret = \"abcdefghijklmnopqrstuvwxyz0123456789\";".to_string(),
        }],
        ..Default::default()
    }
}

fn summary_with(findings: Vec<Finding>) -> ScanSummary {
    ScanSummary {
        metadata: ScanMetadata {
            root_path: PathBuf::from("repo"),
            ..Default::default()
        },
        metrics: Default::default(),
        artifacts: ScanArtifacts {
            findings,
            ..Default::default()
        },
    }
}

fn parse(json: &str) -> serde_json::Value {
    serde_json::from_str(json).expect("AI analysis artifact is valid JSON")
}

#[test]
fn canonical_artifact_uses_shared_finding_and_decision_records() {
    let summary = summary_with(vec![finding(0, Severity::High)]);
    let value = parse(&render_json(
        &summary,
        None,
        &AiContextRenderOptions::default(),
    ));

    assert_eq!(value["schema_version"], AI_CONTEXT_JSON_SCHEMA_VERSION);
    assert_eq!(value["artifact"]["kind"], "repopilot-analysis");
    assert_eq!(value["artifact"]["version"], AI_CONTEXT_ARTIFACT_VERSION);
    assert_eq!(value["artifact"]["source"], "ai-context");
    assert_eq!(
        value["artifact"]["report_schema_version"],
        SCAN_REPORT_SCHEMA_VERSION
    );
    assert_eq!(value["artifact"]["repopilot_version"], REPOPILOT_VERSION);

    let finding = &value["findings"][0];
    assert!(finding["occurrence_key"].is_string());
    assert_eq!(finding["decision"]["severity"], "HIGH");
    assert_eq!(finding["decision"]["confidence"], "HIGH");
    assert!(
        finding["decision"]["verification_plan"]["steps"]
            .as_array()
            .is_some_and(|steps| !steps.is_empty())
    );
    assert_eq!(value["summary"]["verification_plans_included"], 1);
}

#[test]
fn budget_truncates_low_priority_tail_and_reports_counts() {
    let findings = (0..40)
        .map(|index| finding(index, Severity::High))
        .collect();
    let summary = summary_with(findings);
    let budget = 1_200;
    let opts = AiContextRenderOptions {
        focus: None,
        budget_tokens: budget,
        no_header: false,
        no_task: false,
    };

    let value = parse(&render_json(&summary, None, &opts));

    assert_eq!(value["summary"]["findings_total"], 40);
    let included = value["summary"]["findings_included"]
        .as_u64()
        .expect("included count");
    assert!(
        (1..40).contains(&included),
        "should include some but not all findings: {included}"
    );
    assert_eq!(
        included
            + value["summary"]["findings_omitted"]
                .as_u64()
                .expect("omitted count"),
        40
    );
    assert_eq!(value["budget"]["truncated"], true);
    assert_eq!(
        value["findings"].as_array().expect("findings").len() as u64,
        included
    );

    let approx = value["budget"]["approx_tokens"]
        .as_u64()
        .expect("approx tokens");
    assert!(
        approx <= budget as u64,
        "approx tokens {approx} should stay within {budget}"
    );
}

#[test]
fn small_repo_is_not_truncated_and_carries_full_evidence() {
    let summary = summary_with(vec![finding(0, Severity::High)]);
    let value = parse(&render_json(
        &summary,
        None,
        &AiContextRenderOptions::default(),
    ));

    assert_eq!(value["budget"]["truncated"], false);
    assert_eq!(value["summary"]["findings_total"], 1);
    assert_eq!(value["summary"]["findings_included"], 1);
    assert_eq!(value["summary"]["findings_omitted"], 0);

    let finding = &value["findings"][0];
    assert_eq!(finding["rule_id"], "rule.example");
    assert!(finding["risk"]["score"].is_number());
    assert!(
        finding["evidence"][0]["snippet"]
            .as_str()
            .expect("snippet")
            .contains("secret")
    );
    assert_eq!(
        finding["decision"]["recommendation"],
        "Review and fix the example finding."
    );
}

#[test]
fn risk_level_exposes_machine_token_and_display_label() {
    let summary = summary_with(vec![finding(0, Severity::High)]);
    let value = parse(&render_json(
        &summary,
        None,
        &AiContextRenderOptions::default(),
    ));

    let level = value["risk"]["level"].as_str().expect("level");
    let label = value["risk"]["label"].as_str().expect("label");
    assert_eq!(level, level.to_ascii_lowercase());
    assert!(label.to_uppercase().contains(&level.to_uppercase()));
}

#[test]
fn canonical_artifact_is_deterministic() {
    let summary = summary_with(vec![
        finding(1, Severity::Medium),
        finding(0, Severity::High),
    ]);
    let options = AiContextRenderOptions::default();

    assert_eq!(
        render_json(&summary, None, &options),
        render_json(&summary, None, &options)
    );
}
