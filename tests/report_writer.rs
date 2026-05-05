use repopilot::report::writer::write_report;
use std::fs;
use tempfile::tempdir;

#[test]
fn writes_report_to_file() {
    let temp = tempdir().expect("failed to create temp dir");
    let output_path = temp.path().join("report.md");

    write_report("# RepoPilot Report", Some(&output_path)).expect("failed to write report");

    let saved = fs::read_to_string(output_path).expect("failed to read saved report");

    assert_eq!(saved, "# RepoPilot Report");
}

#[test]
fn creates_parent_directories_when_writing_report() {
    let temp = tempdir().expect("failed to create temp dir");
    let output_path = temp.path().join("reports").join("scan").join("report.md");

    write_report("nested report", Some(&output_path)).expect("failed to write nested report");

    let saved = fs::read_to_string(output_path).expect("failed to read saved report");

    assert_eq!(saved, "nested report");
}

#[test]
fn stdout_report_without_output_file_does_not_fail() {
    write_report("console report", None).expect("stdout report should not fail");
}
