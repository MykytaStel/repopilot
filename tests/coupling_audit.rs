use repopilot::findings::types::Severity;
use repopilot::scan::config::ScanConfig;
use repopilot::scan::scanner::scan_path_with_config;
use std::fs;
use std::path::Path;
use tempfile::TempDir;

const COUPLING_RULES: &[&str] = &[
    "architecture.excessive-fan-out",
    "architecture.high-instability-hub",
    "architecture.circular-dependency",
];

#[test]
fn excessive_fan_out_finding_is_emitted() {
    let temp = TempDir::new().expect("failed to create temp dir");
    write_file(
        &temp,
        "src/app.ts",
        r#"
        import { b } from "./b";
        import { c } from "./c";
        import { d } from "./d";

        export function app() { return b() + c() + d(); }
        "#,
    );
    write_file(&temp, "src/b.ts", "export function b() { return 1; }\n");
    write_file(&temp, "src/c.ts", "export function c() { return 1; }\n");
    write_file(&temp, "src/d.ts", "export function d() { return 1; }\n");

    let config = ScanConfig {
        max_fan_out: 2,
        instability_hub_min_fan_in: usize::MAX,
        ..ScanConfig::default()
    };

    let summary = scan_path_with_config(temp.path(), &config).expect("failed to scan temp project");

    let finding = summary
        .artifacts
        .findings
        .iter()
        .find(|finding| finding.rule_id == "architecture.excessive-fan-out")
        .expect("expected excessive fan-out finding");
    assert_eq!(finding.severity, Severity::Medium);
}

#[test]
fn pure_rust_module_facade_does_not_emit_excessive_fan_out() {
    let temp = TempDir::new().expect("failed to create temp dir");
    write_file(
        &temp,
        "src/lib.rs",
        r#"
        pub mod a;
        pub mod b;
        pub mod c;
        "#,
    );
    write_file(&temp, "src/a.rs", "pub fn a() {}\n");
    write_file(&temp, "src/b.rs", "pub fn b() {}\n");
    write_file(&temp, "src/c.rs", "pub fn c() {}\n");

    let config = ScanConfig {
        max_fan_out: 2,
        instability_hub_min_fan_in: usize::MAX,
        ..ScanConfig::default()
    };

    let summary = scan_path_with_config(temp.path(), &config).expect("failed to scan temp project");

    assert!(
        summary.artifacts.findings.iter().all(|finding| {
            finding.rule_id != "architecture.excessive-fan-out"
                || finding.evidence[0].path.as_path() != Path::new("src/lib.rs")
        }),
        "pure Rust facade should not be reported as excessive fan-out: {:#?}",
        summary.artifacts.findings
    );
}

#[test]
fn rust_orchestration_file_still_emits_excessive_fan_out() {
    let temp = TempDir::new().expect("failed to create temp dir");
    write_file(
        &temp,
        "src/lib.rs",
        r#"
        use crate::a::a;
        use crate::b::b;
        use crate::c::c;

        pub mod a;
        pub mod b;
        pub mod c;

        pub fn run() {
            a();
            b();
            c();
        }
        "#,
    );
    write_file(&temp, "src/a.rs", "pub fn a() {}\n");
    write_file(&temp, "src/b.rs", "pub fn b() {}\n");
    write_file(&temp, "src/c.rs", "pub fn c() {}\n");

    let config = ScanConfig {
        max_fan_out: 2,
        instability_hub_min_fan_in: usize::MAX,
        ..ScanConfig::default()
    };

    let summary = scan_path_with_config(temp.path(), &config).expect("failed to scan temp project");

    let finding = summary
        .artifacts
        .findings
        .iter()
        .find(|finding| {
            finding.rule_id == "architecture.excessive-fan-out"
                && finding.evidence[0].path.as_path() == Path::new("src/lib.rs")
        })
        .expect("expected orchestration lib.rs to keep excessive fan-out finding");
    assert_eq!(finding.severity, Severity::Medium);
}

#[test]
fn high_instability_hub_finding_is_emitted() {
    let temp = TempDir::new().expect("failed to create temp dir");
    write_file(
        &temp,
        "src/a.ts",
        r#"
        import { d } from "./d";
        import { e } from "./e";

        export function a() { return d() + e(); }
        "#,
    );
    write_file(
        &temp,
        "src/b.ts",
        r#"import { a } from "./a"; export function b() { return a(); }"#,
    );
    write_file(
        &temp,
        "src/c.ts",
        r#"import { a } from "./a"; export function c() { return a(); }"#,
    );
    write_file(&temp, "src/d.ts", "export function d() { return 1; }\n");
    write_file(&temp, "src/e.ts", "export function e() { return 1; }\n");

    let config = ScanConfig {
        instability_hub_min_fan_in: 2,
        instability_hub_min_instability_pct: 50,
        ..ScanConfig::default()
    };

    let summary = scan_path_with_config(temp.path(), &config).expect("failed to scan temp project");

    let finding = summary
        .artifacts
        .findings
        .iter()
        .find(|finding| finding.rule_id == "architecture.high-instability-hub")
        .expect("expected high-instability hub finding");
    assert_eq!(finding.severity, Severity::High);
}

#[test]
fn circular_dependency_finding_is_emitted() {
    let temp = TempDir::new().expect("failed to create temp dir");
    write_file(
        &temp,
        "src/a.ts",
        r#"import { b } from "./b"; export function a() { return b(); }"#,
    );
    write_file(
        &temp,
        "src/b.ts",
        r#"import { a } from "./a"; export function b() { return a(); }"#,
    );

    let summary = scan_path_with_config(temp.path(), &ScanConfig::default())
        .expect("failed to scan temp project");

    let finding = summary
        .artifacts
        .findings
        .iter()
        .find(|finding| finding.rule_id == "architecture.circular-dependency")
        .expect("expected circular dependency finding");
    assert_eq!(finding.severity, Severity::High);
}

#[test]
fn isolated_files_do_not_emit_coupling_findings() {
    let temp = TempDir::new().expect("failed to create temp dir");
    write_file(&temp, "src/a.ts", "export function a() { return 1; }\n");
    write_file(&temp, "src/b.ts", "export function b() { return 1; }\n");

    let summary = scan_path_with_config(temp.path(), &ScanConfig::default())
        .expect("failed to scan temp project");

    assert!(
        summary
            .artifacts
            .findings
            .iter()
            .all(|finding| !COUPLING_RULES.contains(&finding.rule_id.as_str()))
    );
}

#[test]
fn scan_summary_stores_coupling_graph() {
    let temp = TempDir::new().expect("failed to create temp dir");
    write_file(&temp, "src/a.ts", "export function a() { return 1; }\n");

    let summary = scan_path_with_config(temp.path(), &ScanConfig::default())
        .expect("failed to scan temp project");

    assert!(summary.artifacts.coupling_graph.is_some());
}

fn write_file(temp: &TempDir, relative_path: &str, content: &str) {
    let path = temp.path().join(relative_path);
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).expect("failed to create parent dir");
    }
    fs::write(path, content).expect("failed to write file");
}
