use repopilot::audits::code_quality::deep_control_flow::DeepControlFlowAudit;
use repopilot::audits::traits::FileAudit;
use repopilot::findings::types::Severity;
use repopilot::scan::config::ScanConfig;
use repopilot::scan::facts::FileFacts;
use std::path::PathBuf;

fn file_facts(path: &str, language: &str, content: &str) -> FileFacts {
    FileFacts {
        path: PathBuf::from(path),
        language: Some(language.to_string()),
        non_empty_lines: content.lines().count(),
        branch_count: 0,
        imports: Vec::new(),
        content: Some(content.to_string()),
        has_inline_tests: false,
    }
}

#[test]
fn rust_detects_exceeded_nesting_depth() {
    let content = r#"
        fn main() {
            if a { // depth 1
                for x in y { // depth 2
                    if b { // depth 3
                        match z { // depth 4
                            Some(w) => {
                                if w { // depth 5 (exceeds threshold 4)
                                    println!("deep");
                                }
                            }
                        }
                    }
                }
            }
        }
    "#;

    let file = file_facts("src/main.rs", "Rust", content);
    let config = ScanConfig {
        max_control_flow_depth: 4,
        ..ScanConfig::default()
    };

    let findings = DeepControlFlowAudit.audit(&file, &config);

    assert_eq!(findings.len(), 1);
    assert_eq!(findings[0].rule_id, "code-quality.deep-control-flow");
    assert_eq!(findings[0].severity, Severity::Low);
    assert_eq!(findings[0].evidence[0].line_start, 8); // line of `if w {`
    assert!(findings[0].evidence[0].snippet.contains("if w"));
}

#[test]
fn rust_flattens_else_if_chains() {
    let content = r#"
        fn main() {
            if a {
            } else if b {
            } else if c {
            } else if d {
            } else if e {
            }
        }
    "#;

    let file = file_facts("src/main.rs", "Rust", content);
    let config = ScanConfig {
        max_control_flow_depth: 2,
        ..ScanConfig::default()
    };

    let findings = DeepControlFlowAudit.audit(&file, &config);

    assert!(findings.is_empty(), "else if chains should be flat");
}

#[test]
fn ts_detects_exceeded_nesting_depth() {
    let content = r#"
        function test() {
            if (a) { // depth 1
                while (b) { // depth 2
                    if (c) { // depth 3
                        switch (d) { // depth 4
                            case 1:
                                if (e) { // depth 5 (exceeds threshold 4)
                                    console.log("nested");
                                }
                                break;
                        }
                    }
                }
            }
        }
    "#;

    let file = file_facts("src/index.ts", "TypeScript", content);
    let config = ScanConfig {
        max_control_flow_depth: 4,
        ..ScanConfig::default()
    };

    let findings = DeepControlFlowAudit.audit(&file, &config);

    assert_eq!(findings.len(), 1);
    assert_eq!(findings[0].rule_id, "code-quality.deep-control-flow");
    assert_eq!(findings[0].evidence[0].line_start, 8); // line of `if (e) {`
    assert!(findings[0].evidence[0].snippet.contains("if (e)"));
}

#[test]
fn ts_flattens_else_if_chains() {
    let content = r#"
        function test() {
            if (a) {
            } else if (b) {
            } else if (c) {
            } else if (d) {
            }
        }
    "#;

    let file = file_facts("src/index.ts", "TypeScript", content);
    let config = ScanConfig {
        max_control_flow_depth: 2,
        ..ScanConfig::default()
    };

    let findings = DeepControlFlowAudit.audit(&file, &config);

    assert!(findings.is_empty(), "JS/TS else if chains should be flat");
}

#[test]
fn python_detects_exceeded_nesting_depth() {
    let content = r#"
def test():
    if a: # depth 1
        for b in c: # depth 2
            if d: # depth 3
                try: # depth 4
                    if e: # depth 5 (exceeds threshold 4)
                        print("nested")
                except Exception:
                    pass
"#;

    let file = file_facts("src/main.py", "Python", content);
    let config = ScanConfig {
        max_control_flow_depth: 4,
        ..ScanConfig::default()
    };

    let findings = DeepControlFlowAudit.audit(&file, &config);

    assert_eq!(findings.len(), 1);
    assert_eq!(findings[0].rule_id, "code-quality.deep-control-flow");
    assert_eq!(findings[0].evidence[0].line_start, 7); // line of `if e:`
    assert!(findings[0].evidence[0].snippet.contains("if e:"));
}
