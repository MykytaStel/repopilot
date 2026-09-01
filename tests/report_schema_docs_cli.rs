use repopilot::report::schema::SCAN_REPORT_SCHEMA_VERSION;
use serde_json::Value;
use std::collections::BTreeMap;
use std::fs;
use std::path::Path;
use std::process::{Command, Output};

const REPORTS_GUIDE: &str = include_str!("../docs/reports.md");

#[test]
fn release_022_schema_table_preserves_the_tagged_contract() {
    // Frozen source contract from v0.22.0 (a000dc7cf189798b814672673076cf9179b1a267),
    // src/report/schema.rs and src/receipt/model.rs. Do not use the current schema
    // constant here: future revisions must not rewrite a shipped release's docs.
    // This guards historical declarations, not old-binary compatibility.
    let notes = include_str!("../docs/releases/v0.22.0.md");
    let rows: Vec<_> = notes
        .lines()
        .filter(|line| line.starts_with("| `"))
        .map(|line| {
            let cells: Vec<_> = line.split('|').map(str::trim).collect();
            assert_eq!(cells.len(), 4, "expected artifact/schema table row");
            (cells[1].trim_matches('`'), cells[2].trim_matches('`'))
        })
        .collect();
    assert_eq!(
        rows.len(),
        5,
        "one declaration per v0.22 report/reader surface"
    );
    assert_eq!(
        rows.into_iter().collect::<BTreeMap<_, _>>(),
        BTreeMap::from([
            ("scan", "0.26"),
            ("baseline-scan", "0.26"),
            ("review", "0.26"),
            ("receipt", "6"),
            ("scan-reader input", "0.16–0.26"),
        ]),
        "release declarations must preserve the v0.22.0 tagged source contract"
    );
}

fn run_ok(program: &str, args: &[&str], root: &Path) -> Output {
    let output = Command::new(program)
        .args(args)
        .current_dir(root)
        .output()
        .expect("run command");
    assert!(
        output.status.success(),
        "{program} {args:?} failed: {}\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    output
}

fn cli(args: &[&str], root: &Path) -> Output {
    run_ok(env!("CARGO_BIN_EXE_repopilot"), args, root)
}

fn repository() -> tempfile::TempDir {
    let repo = tempfile::tempdir().expect("temporary repository");
    fs::write(repo.path().join("main.py"), "print('before')\n").unwrap();
    run_ok("git", &["init", "-q"], repo.path());
    run_ok("git", &["add", "main.py"], repo.path());
    run_ok(
        "git",
        &[
            "-c",
            "user.name=Schema Test",
            "-c",
            "user.email=schema@example.invalid",
            "-c",
            "commit.gpgsign=false",
            "-c",
            "core.hooksPath=/dev/null",
            "commit",
            "-qm",
            "initial source",
        ],
        repo.path(),
    );
    repo
}

fn documented_report(kind: &str) -> Value {
    let examples: Vec<Value> = REPORTS_GUIDE
        .split("```json\n")
        .skip(1)
        .map(|block| {
            serde_json::from_str(block.split("```").next().unwrap())
                .expect("reports guide JSON example must parse")
        })
        .filter(|value: &Value| value["report"]["kind"] == kind)
        .collect();
    assert_eq!(examples.len(), 1, "expected one {kind} envelope example");
    examples.into_iter().next().unwrap()
}

fn assert_documented_envelope(output: Output, kind: &str) -> Value {
    let emitted: Value = serde_json::from_slice(&output.stdout).expect("CLI JSON report");
    assert_eq!(emitted["report"]["kind"], kind);
    assert_eq!(emitted["schema_version"], SCAN_REPORT_SCHEMA_VERSION);
    assert_eq!(
        emitted["report"]["schema_version"],
        emitted["schema_version"]
    );
    assert_eq!(emitted["repopilot_version"], env!("CARGO_PKG_VERSION"));
    assert_eq!(
        emitted["report"]["repopilot_version"],
        emitted["repopilot_version"]
    );

    let documented = documented_report(kind);
    for key in ["schema_version", "repopilot_version", "report"] {
        assert_eq!(
            documented[key], emitted[key],
            "docs/reports.md {kind}.{key} must match emitted CLI metadata"
        );
    }
    emitted
}

#[test]
fn scan_envelope_matches_documented_json() {
    let repo = repository();
    let report =
        assert_documented_envelope(cli(&["scan", ".", "--format", "json"], repo.path()), "scan");
    assert_eq!(report["files_analyzed"], 1);
    assert_eq!(report["assessment_status"], "assessed");
}

#[test]
fn baseline_scan_envelope_matches_documented_json() {
    let repo = repository();
    cli(
        &[
            "baseline",
            "create",
            ".",
            "--output",
            ".repopilot/baseline.json",
        ],
        repo.path(),
    );
    let report = assert_documented_envelope(
        cli(
            &[
                "scan",
                ".",
                "--baseline",
                ".repopilot/baseline.json",
                "--format",
                "json",
            ],
            repo.path(),
        ),
        "baseline-scan",
    );
    assert_eq!(report["files_analyzed"], 1);
    assert_eq!(report["assessment_status"], "assessed");
    assert!(report["baseline"].is_object());
}

#[test]
fn nonempty_review_envelope_matches_documented_json() {
    let repo = repository();
    fs::write(repo.path().join("main.py"), "print('after')\n").unwrap();
    let report = assert_documented_envelope(
        cli(&["review", ".", "--format", "json"], repo.path()),
        "review",
    );
    assert_eq!(report["files_analyzed"], 1);
    assert_eq!(report["changed_files"].as_array().unwrap().len(), 1);
}
