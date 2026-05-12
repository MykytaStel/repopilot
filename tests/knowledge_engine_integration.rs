use repopilot::findings::types::Severity;
use repopilot::scan::config::ScanConfig;
use repopilot::scan::scanner::scan_path_with_config;
use std::fs;
use tempfile::tempdir;

#[test]
fn polyglot_scan_uses_knowledge_engine_context_without_flagging_paradigms() {
    let temp = tempdir().expect("temp dir");
    let root = temp.path();

    write(
        root.join("src/domain/user.rs"),
        "pub fn user_name() -> String { panic!(\"invalid domain state\") }\n",
    );
    write(
        root.join("src/main.rs"),
        "fn main() { let config = load_config().unwrap(); }\nfn load_config() -> Result<(), ()> { Ok(()) }\n",
    );
    write(
        root.join("src/users.ts"),
        "const names = users.filter(user => user.active).map(user => user.name);\n",
    );
    write(
        root.join("app/views.py"),
        "from fastapi import FastAPI\napp = FastAPI()\n",
    );

    let summary = scan_path_with_config(
        root,
        &ScanConfig {
            detect_missing_tests: false,
            ..ScanConfig::default()
        },
    )
    .expect("scan");

    assert!(
        summary
            .languages
            .iter()
            .any(|language| language.name == "Rust")
    );
    assert!(
        summary
            .languages
            .iter()
            .any(|language| language.name == "TypeScript")
    );
    assert!(
        summary
            .languages
            .iter()
            .any(|language| language.name == "Python")
    );

    let rust_findings: Vec<_> = summary
        .findings
        .iter()
        .filter(|finding| finding.rule_id == "language.rust.panic-risk")
        .collect();

    assert_eq!(rust_findings.len(), 2);
    assert!(
        rust_findings
            .iter()
            .any(|finding| finding.severity == Severity::High)
    );
    assert!(
        rust_findings
            .iter()
            .any(|finding| finding.severity == Severity::Low)
    );
    assert!(
        summary
            .findings
            .iter()
            .all(|finding| !finding.title.to_lowercase().contains("functional"))
    );
}

#[test]
fn node_cli_boundary_downgrades_process_exit() {
    let temp = tempdir().expect("temp dir");
    let root = temp.path();

    write(root.join("src/main.js"), "process.exit(1);\n");

    let summary = scan_path_with_config(
        root,
        &ScanConfig {
            detect_missing_tests: false,
            ..ScanConfig::default()
        },
    )
    .expect("scan");

    let finding = summary
        .findings
        .iter()
        .find(|finding| finding.rule_id == "language.javascript.runtime-exit-risk")
        .expect("process exit finding");

    assert_eq!(finding.severity, Severity::Low);
}

#[test]
fn generated_files_suppress_language_risk_noise() {
    let temp = tempdir().expect("temp dir");
    let root = temp.path();

    write(
        root.join("src/generated/client.go"),
        "package generated\nfunc Parse() { panic(\"generated\") }\n",
    );

    let summary = scan_path_with_config(
        root,
        &ScanConfig {
            detect_missing_tests: false,
            ..ScanConfig::default()
        },
    )
    .expect("scan");

    assert!(
        summary
            .findings
            .iter()
            .all(|finding| finding.rule_id != "language.go.panic-exit-risk")
    );
}

#[test]
fn no_audit_uses_hardcoded_language_allowlist_for_applicability() {
    let root = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let source_files = [
        "src/audits/architecture/large_file.rs",
        "src/audits/code_quality/complexity.rs",
        "src/audits/code_quality/long_function/mod.rs",
    ];

    for relative_path in source_files {
        let content = fs::read_to_string(root.join(relative_path)).expect("read source file");
        for forbidden in ["fn is_supported", "fn is_code_language", "fn is_code_file"] {
            assert!(
                !content.contains(forbidden),
                "{relative_path} still contains local applicability helper `{forbidden}`"
            );
        }
    }
}

fn write(path: std::path::PathBuf, content: &str) {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).expect("create parent dir");
    }
    fs::write(path, content).expect("write fixture");
}
