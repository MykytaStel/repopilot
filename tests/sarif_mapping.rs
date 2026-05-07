use repopilot::findings::types::{Evidence, Finding, FindingCategory, Severity};
use repopilot::output::sarif::{findings_to_sarif, scan_summary_to_sarif};
use repopilot::scan::types::ScanSummary;
use std::path::{Path, PathBuf};

#[test]
fn scan_summary_maps_to_minimal_sarif_log() {
    let summary = ScanSummary {
        root_path: PathBuf::from("/repo"),
        findings: vec![finding(
            "architecture.large-file",
            Severity::High,
            Some("/repo/src/main.rs"),
            12,
        )],
        ..ScanSummary::default()
    };

    let sarif = scan_summary_to_sarif(&summary, Path::new("/repo"));

    assert_eq!(sarif.version, "2.1.0");
    assert_eq!(
        sarif.schema,
        "https://json.schemastore.org/sarif-2.1.0.json"
    );
    assert_eq!(sarif.runs.len(), 1);
    assert_eq!(sarif.runs[0].tool.driver.name, "RepoPilot");
    assert_eq!(
        sarif.runs[0].tool.driver.information_uri,
        "https://github.com/MykytaStel/repopilot"
    );
    assert_eq!(sarif.runs[0].results[0].rule_id, "architecture.large-file");
    assert_eq!(sarif.runs[0].results[0].message.text, "Finding title");
}

#[test]
fn maps_severity_to_sarif_level() {
    let sarif = findings_to_sarif(
        &[
            finding("rule.high", Severity::High, Some("src/high.rs"), 1),
            finding(
                "rule.critical",
                Severity::Critical,
                Some("src/critical.rs"),
                1,
            ),
            finding("rule.medium", Severity::Medium, Some("src/medium.rs"), 1),
            finding("rule.low", Severity::Low, Some("src/low.rs"), 1),
            finding("rule.info", Severity::Info, Some("src/info.rs"), 1),
        ],
        Path::new("."),
    );

    let levels = sarif.runs[0]
        .results
        .iter()
        .map(|result| result.level.as_str())
        .collect::<Vec<_>>();

    assert_eq!(levels, ["error", "error", "warning", "note", "note"]);
}

#[test]
fn duplicate_rule_ids_produce_one_sorted_sarif_rule() {
    let sarif = findings_to_sarif(
        &[
            finding(
                "security.secret-candidate",
                Severity::High,
                Some("src/a.rs"),
                1,
            ),
            finding("code-marker.todo", Severity::Low, Some("src/b.rs"), 2),
            finding(
                "security.secret-candidate",
                Severity::High,
                Some("src/c.rs"),
                3,
            ),
        ],
        Path::new("."),
    );

    let rules = &sarif.runs[0].tool.driver.rules;

    assert_eq!(rules.len(), 2);
    assert_eq!(rules[0].id, "code-marker.todo");
    assert_eq!(rules[0].name, "code-marker.todo");
    assert_eq!(
        rules[0].short_description.text, "Finding description",
        "rule shortDescription should come from the finding description"
    );
    assert_eq!(rules[1].id, "security.secret-candidate");
}

#[test]
fn finding_path_is_emitted_as_relative_forward_slash_uri() {
    let sarif = findings_to_sarif(
        &[finding(
            "code-marker.todo",
            Severity::Low,
            Some("/repo/src/main.rs"),
            7,
        )],
        Path::new("/repo"),
    );

    let location = &sarif.runs[0].results[0].locations[0].physical_location;

    assert_eq!(location.artifact_location.uri, "src/main.rs");
    assert_eq!(
        location.region.as_ref().map(|region| region.start_line),
        Some(7)
    );
}

#[test]
fn finding_without_path_has_no_locations() {
    let sarif = findings_to_sarif(
        &[finding("architecture.deep-nesting", Severity::Low, None, 0)],
        Path::new("."),
    );

    assert!(sarif.runs[0].results[0].locations.is_empty());
}

#[test]
fn empty_locations_are_omitted_from_serialized_result() {
    let sarif = findings_to_sarif(
        &[finding("architecture.deep-nesting", Severity::Low, None, 0)],
        Path::new("."),
    );

    let json = serde_json::to_value(&sarif).expect("SARIF should serialize");

    assert!(json["runs"][0]["results"][0].get("locations").is_none());
}

fn finding(rule_id: &str, severity: Severity, path: Option<&str>, line_start: usize) -> Finding {
    Finding {
        id: format!("{rule_id}:1"),
        rule_id: rule_id.to_string(),
        title: "Finding title".to_string(),
        description: "Finding description".to_string(),
        category: FindingCategory::Architecture,
        severity,
        evidence: path
            .map(|path| {
                vec![Evidence {
                    path: PathBuf::from(path),
                    line_start,
                    line_end: None,
                    snippet: "evidence".to_string(),
                }]
            })
            .unwrap_or_default(),
    }
}
