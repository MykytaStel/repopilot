use repopilot::baseline::reader::read_baseline;
use std::fs;
use tempfile::tempdir;

#[test]
fn reads_valid_baseline() {
    let temp = tempdir().expect("failed to create temp dir");
    let path = temp.path().join("baseline.json");
    fs::write(&path, valid_baseline_json()).expect("failed to write baseline");

    let baseline = read_baseline(&path).expect("baseline should read");

    assert_eq!(baseline.schema_version, 1);
    assert_eq!(baseline.tool, "repopilot");
    assert_eq!(
        baseline.findings[0].key,
        "security.secret-candidate:src/config.rs:1"
    );
}

#[test]
fn rejects_invalid_json() {
    let temp = tempdir().expect("failed to create temp dir");
    let path = temp.path().join("baseline.json");
    fs::write(&path, "{not json").expect("failed to write baseline");

    let error = read_baseline(&path).expect_err("invalid JSON should fail");

    assert!(error.to_string().contains("invalid JSON"));
}

#[test]
fn rejects_empty_baseline_file() {
    let temp = tempdir().expect("failed to create temp dir");
    let path = temp.path().join("baseline.json");
    fs::write(&path, "\n").expect("failed to write baseline");

    let error = read_baseline(&path).expect_err("empty baseline should fail");

    assert!(error.to_string().contains("baseline file is empty"));
}

#[test]
fn rejects_unsupported_schema_version() {
    let temp = tempdir().expect("failed to create temp dir");
    let path = temp.path().join("baseline.json");
    fs::write(
        &path,
        valid_baseline_json().replace("\"schema_version\": 1", "\"schema_version\": 99"),
    )
    .expect("failed to write baseline");

    let error = read_baseline(&path).expect_err("unsupported schema should fail");

    assert!(
        error
            .to_string()
            .contains("Unsupported baseline schema version: 99")
    );
    assert!(error.to_string().contains("Supported version: 1"));
}

#[test]
fn handles_missing_file_with_clear_error() {
    let temp = tempdir().expect("failed to create temp dir");
    let path = temp.path().join("missing.json");

    let error = read_baseline(&path).expect_err("missing file should fail");

    assert!(error.to_string().contains("Failed to read baseline file"));
    assert!(error.to_string().contains("file does not exist"));
}

#[test]
fn rejects_finding_missing_required_field() {
    let temp = tempdir().expect("failed to create temp dir");
    let path = temp.path().join("baseline.json");
    fs::write(
        &path,
        valid_baseline_json().replace(
            "\"key\": \"security.secret-candidate:src/config.rs:1\",\n      ",
            "",
        ),
    )
    .expect("failed to write baseline");

    let error = read_baseline(&path).expect_err("missing finding key should fail");

    assert!(error.to_string().contains("key"));
}

fn valid_baseline_json() -> String {
    r#"{
  "schema_version": 1,
  "tool": "repopilot",
  "created_at": "2026-05-06T12:00:00Z",
  "root": ".",
  "findings": [
    {
      "key": "security.secret-candidate:src/config.rs:1",
      "rule_id": "security.secret-candidate",
      "severity": "high",
      "path": "src/config.rs",
      "message": "Possible secret detected"
    }
  ]
}"#
    .to_string()
}
