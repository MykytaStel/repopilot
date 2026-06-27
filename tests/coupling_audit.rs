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

    summary
        .artifacts
        .findings
        .iter()
        .find(|finding| finding.rule_id == "architecture.excessive-fan-out")
        .expect("expected excessive fan-out finding");
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

    summary
        .artifacts
        .findings
        .iter()
        .find(|finding| {
            finding.rule_id == "architecture.excessive-fan-out"
                && finding.evidence[0].path.as_path() == Path::new("src/lib.rs")
        })
        .expect("expected orchestration lib.rs to keep excessive fan-out finding");
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

    summary
        .artifacts
        .findings
        .iter()
        .find(|finding| finding.rule_id == "architecture.high-instability-hub")
        .expect("expected high-instability hub finding");
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
    // Evidence lists the cycle members with repository-relative paths.
    assert!(!finding.evidence.is_empty());
    let cycle_files: Vec<_> = finding
        .evidence
        .iter()
        .map(|item| {
            assert!(
                item.path.is_relative(),
                "evidence path should be relative, got {:?}",
                item.path
            );
            item.path.as_path()
        })
        .collect();
    assert!(cycle_files.contains(&Path::new("src/a.ts")));
    assert!(cycle_files.contains(&Path::new("src/b.ts")));
}

#[test]
fn inline_type_only_cycle_does_not_emit_circular_dependency_even_in_strict() {
    let temp = TempDir::new().expect("failed to create temp dir");
    write_file(
        &temp,
        "src/a.ts",
        r#"import { type B } from "./b"; export type A = { b?: B };"#,
    );
    write_file(
        &temp,
        "src/b.ts",
        r#"import { type A } from "./a"; export type B = { a?: A };"#,
    );

    let config = ScanConfig {
        include_low_signal: true,
        ..ScanConfig::default()
    };
    let summary = scan_path_with_config(temp.path(), &config).expect("failed to scan temp project");

    assert!(
        summary
            .artifacts
            .findings
            .iter()
            .all(|finding| finding.rule_id != "architecture.circular-dependency"),
        "inline type-only cycle must not be reported even in strict visibility: {:#?}",
        summary.artifacts.findings
    );
}

#[test]
fn acyclic_imports_do_not_emit_circular_dependency() {
    let temp = TempDir::new().expect("failed to create temp dir");
    write_file(
        &temp,
        "src/a.ts",
        r#"import { b } from "./b"; export function a() { return b(); }"#,
    );
    write_file(
        &temp,
        "src/b.ts",
        r#"import { c } from "./c"; export function b() { return c(); }"#,
    );
    write_file(&temp, "src/c.ts", "export function c() { return 1; }\n");

    let summary = scan_path_with_config(temp.path(), &ScanConfig::default())
        .expect("failed to scan temp project");

    assert!(
        summary
            .artifacts
            .findings
            .iter()
            .all(|finding| finding.rule_id != "architecture.circular-dependency"),
        "acyclic import chain must not be reported as circular: {:#?}",
        summary.artifacts.findings
    );
}

#[test]
fn deferred_only_python_cycle_is_low_severity() {
    let temp = TempDir::new().expect("failed to create temp dir");
    write_file(&temp, "app/__init__.py", "");
    write_file(
        &temp,
        "app/models.py",
        "import app.views\nthing = object()\n",
    );
    write_file(
        &temp,
        "app/views.py",
        "def handler():\n    from app.models import thing\n    return thing\n",
    );

    let summary = scan_path_with_config(temp.path(), &ScanConfig::default())
        .expect("failed to scan temp project");

    let finding = summary
        .artifacts
        .findings
        .iter()
        .find(|finding| finding.rule_id == "architecture.circular-dependency")
        .expect("expected strict-visible deferred cycle finding");
    assert_eq!(finding.severity, Severity::Low);
    assert!(
        finding.description.contains("deferred-only"),
        "deferred-only cycle should explain its lower severity: {}",
        finding.description
    );
}

#[test]
fn mixed_scc_with_eager_cycle_emits_only_high_finding() {
    // Eager cycle a <-> b, plus a deferred edge b -> c and an eager edge c -> a.
    // The deferred edge widens the full-graph SCC to {a, b, c}, but that component
    // is NOT deferred-only: it still contains the eager a <-> b cycle. The fix
    // must report a single High finding for a <-> b and no misleading Low
    // "deferred-only" finding for {a, b, c}.
    let temp = TempDir::new().expect("failed to create temp dir");
    write_file(&temp, "app/__init__.py", "");
    write_file(&temp, "app/a.py", "import app.b\n");
    write_file(
        &temp,
        "app/b.py",
        "import app.a\n\n\ndef use():\n    import app.c\n    return app.c\n",
    );
    write_file(&temp, "app/c.py", "import app.a\n");

    let summary = scan_path_with_config(temp.path(), &ScanConfig::default())
        .expect("failed to scan temp project");

    let circular: Vec<_> = summary
        .artifacts
        .findings
        .iter()
        .filter(|finding| finding.rule_id == "architecture.circular-dependency")
        .collect();

    assert_eq!(
        circular.len(),
        1,
        "mixed component must yield exactly one High finding, no deferred-only Low: {circular:#?}"
    );
    assert_eq!(circular[0].severity, Severity::High);
    assert!(
        !circular[0].description.contains("deferred-only"),
        "the eager cycle must not be mislabelled as deferred-only: {}",
        circular[0].description
    );
}

#[test]
fn multiple_circular_dependencies_are_each_reported() {
    let temp = TempDir::new().expect("failed to create temp dir");
    // Two independent 2-file cycles: a <-> b and c <-> d.
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
    write_file(
        &temp,
        "src/c.ts",
        r#"import { d } from "./d"; export function c() { return d(); }"#,
    );
    write_file(
        &temp,
        "src/d.ts",
        r#"import { c } from "./c"; export function d() { return c(); }"#,
    );

    let summary = scan_path_with_config(temp.path(), &ScanConfig::default())
        .expect("failed to scan temp project");

    let circular: Vec<_> = summary
        .artifacts
        .findings
        .iter()
        .filter(|finding| finding.rule_id == "architecture.circular-dependency")
        .collect();

    // One finding per strongly-connected component.
    assert_eq!(
        circular.len(),
        2,
        "expected one finding per cycle: {circular:#?}"
    );
    assert!(
        circular
            .iter()
            .all(|finding| finding.severity == Severity::High)
    );
}

#[test]
fn circular_dependency_finding_leads_with_the_minimal_cycle() {
    let temp = TempDir::new().expect("failed to create temp dir");
    // One 4-file component {a,b,c,d}; the minimal cycle is a -> b -> c -> a,
    // with d reachable through the c -> d -> a shortcut.
    write_file(
        &temp,
        "src/a.ts",
        r#"import { b } from "./b"; export function a() { return b(); }"#,
    );
    write_file(
        &temp,
        "src/b.ts",
        r#"import { c } from "./c"; export function b() { return c(); }"#,
    );
    write_file(
        &temp,
        "src/c.ts",
        r#"import { a } from "./a"; import { d } from "./d"; export function c() { return a() + d(); }"#,
    );
    write_file(
        &temp,
        "src/d.ts",
        r#"import { a } from "./a"; export function d() { return a(); }"#,
    );

    let summary = scan_path_with_config(temp.path(), &ScanConfig::default())
        .expect("failed to scan temp project");

    let circular: Vec<_> = summary
        .artifacts
        .findings
        .iter()
        .filter(|finding| finding.rule_id == "architecture.circular-dependency")
        .collect();
    assert_eq!(circular.len(), 1, "expected one finding per component");
    let finding = circular[0];

    // The headline is the minimal cycle, not the whole component.
    assert!(
        finding
            .description
            .contains("src/a.ts -> src/b.ts -> src/c.ts -> src/a.ts"),
        "description should lead with the minimal cycle, got: {}",
        finding.description
    );
    // Component size is carried as context.
    assert!(
        finding
            .description
            .contains("strongly-connected component of 4 files"),
        "description should report component size, got: {}",
        finding.description
    );
    // Evidence covers only the minimal cycle's distinct files (a, b, c) — not d.
    let evidence_paths: Vec<_> = finding
        .evidence
        .iter()
        .map(|item| item.path.as_path())
        .collect();
    assert_eq!(evidence_paths.len(), 3, "evidence: {evidence_paths:?}");
    assert!(evidence_paths.contains(&Path::new("src/a.ts")));
    assert!(evidence_paths.contains(&Path::new("src/b.ts")));
    assert!(evidence_paths.contains(&Path::new("src/c.ts")));
    assert!(!evidence_paths.contains(&Path::new("src/d.ts")));
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
