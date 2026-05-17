use serde_json::Value;
use std::fs;
use std::process::Command;
use tempfile::tempdir;

fn repopilot() -> Command {
    Command::new(env!("CARGO_BIN_EXE_repopilot"))
}

#[test]
fn default_scan_hides_source_without_test_but_strict_includes_it() {
    let temp = tempdir().expect("temp dir");
    let root = temp.path();
    write(
        root.join("src/payment.rs"),
        "pub fn charge() -> bool { true }\n",
    );

    let default = scan_json(root, &[]);
    assert_rule_absent(&default, "testing.source-without-test");
    assert!(
        default["hidden_suggestions_count"].as_u64().unwrap_or(0) >= 1,
        "default report should count hidden testing suggestions: {default:#?}"
    );

    let strict = scan_json(root, &["--profile", "strict"]);
    assert_rule_present(&strict, "testing.source-without-test");
}

#[test]
fn default_scan_hides_script_process_exit_but_reports_library_process_exit() {
    let temp = tempdir().expect("temp dir");
    let root = temp.path();
    write(root.join("scripts/check.js"), "process.exit(1);\n");
    write(
        root.join("src/lib/runtime.js"),
        "export function stop() { process.exit(1); }\n",
    );

    let json = scan_json(root, &[]);
    let runtime_findings =
        findings_for_rule(&json, "language.javascript.runtime-exit-risk").collect::<Vec<_>>();

    assert_eq!(
        runtime_findings.len(),
        1,
        "default report should hide script process.exit and keep library process.exit: {json:#?}"
    );
    assert!(
        first_path(runtime_findings[0]).ends_with("src/lib/runtime.js"),
        "reported process.exit should be in reusable library code"
    );
}

#[test]
fn default_scan_reports_unwrap_on_external_parse_path() {
    let temp = tempdir().expect("temp dir");
    let root = temp.path();
    write(
        root.join("src/parser.rs"),
        "pub fn parse_port(raw: &str) -> u16 { raw.parse::<u16>().unwrap() }\n",
    );

    let json = scan_json(root, &[]);
    let rust_findings = findings_for_rule(&json, "language.rust.panic-risk").collect::<Vec<_>>();

    assert_eq!(
        rust_findings.len(),
        1,
        "external parse unwrap should remain visible in default report: {json:#?}"
    );
    assert_eq!(rust_findings[0]["severity"], "HIGH");
}

fn scan_json(root: &std::path::Path, extra_args: &[&str]) -> Value {
    let output = repopilot()
        .args(["scan", ".", "--format", "json"])
        .args(extra_args)
        .current_dir(root)
        .output()
        .expect("run repopilot scan");

    assert!(
        output.status.success(),
        "scan failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );

    serde_json::from_slice(&output.stdout).expect("json output")
}

fn assert_rule_present(json: &Value, rule_id: &str) {
    assert!(
        findings_for_rule(json, rule_id).next().is_some(),
        "expected {rule_id} in findings: {json:#?}"
    );
}

fn assert_rule_absent(json: &Value, rule_id: &str) {
    assert!(
        findings_for_rule(json, rule_id).next().is_none(),
        "did not expect {rule_id} in findings: {json:#?}"
    );
}

fn findings_for_rule<'a>(
    json: &'a Value,
    rule_id: &'a str,
) -> impl Iterator<Item = &'a Value> + 'a {
    json["findings"]
        .as_array()
        .into_iter()
        .flatten()
        .filter(move |finding| finding["rule_id"] == rule_id)
}

fn first_path(finding: &Value) -> &str {
    finding["evidence"][0]["path"]
        .as_str()
        .expect("finding path")
}

fn write(path: std::path::PathBuf, content: &str) {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).expect("create parent");
    }
    fs::write(path, content).expect("write file");
}
