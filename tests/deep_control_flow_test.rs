use repopilot::audits::code_quality::deep_control_flow::DeepControlFlowAudit;
use repopilot::audits::traits::FileAudit;
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

// ── Go ────────────────────────────────────────────────────────────────────────

#[test]
fn go_detects_exceeded_nesting_depth() {
    let content = r#"
        package main
        func main() {
            if a { // depth 1
                for b { // depth 2
                    if c { // depth 3
                        switch d { // depth 4
                        case 1:
                            if e { // depth 5 (exceeds threshold 4)
                                println("nested")
                            }
                        }
                    }
                }
            }
        }
    "#;

    let file = file_facts("src/main.go", "Go", content);
    let config = ScanConfig {
        max_control_flow_depth: 4,
        ..ScanConfig::default()
    };

    let findings = DeepControlFlowAudit.audit(&file, &config);

    assert_eq!(findings.len(), 1);
    assert_eq!(findings[0].evidence[0].line_start, 9); // line of `if e {`
    assert!(findings[0].evidence[0].snippet.contains("if e"));
}

// ── Java ──────────────────────────────────────────────────────────────────────

#[test]
fn java_detects_exceeded_nesting_depth() {
    let content = r#"
        class Test {
            void test() {
                if (a) { // depth 1
                    for (int x : y) { // depth 2 (enhanced_for_statement)
                        if (b) { // depth 3
                            try { // depth 4
                                if (c) { // depth 5 (exceeds threshold 4)
                                    System.out.println("nested");
                                }
                            } catch (Exception e) {}
                        }
                    }
                }
            }
        }
    "#;

    let file = file_facts("src/App.java", "Java", content);
    let config = ScanConfig {
        max_control_flow_depth: 4,
        ..ScanConfig::default()
    };

    let findings = DeepControlFlowAudit.audit(&file, &config);

    assert_eq!(findings.len(), 1);
    assert_eq!(findings[0].evidence[0].line_start, 8); // line of `if (c) {`
    assert!(findings[0].evidence[0].snippet.contains("if (c)"));
}

// ── C# ────────────────────────────────────────────────────────────────────────

#[test]
fn csharp_detects_exceeded_nesting_depth() {
    let content = r#"
        class Test {
            void TestMethod() {
                if (a) { // depth 1
                    foreach (var x in y) { // depth 2
                        if (b) { // depth 3
                            try { // depth 4
                                if (c) { // depth 5 (exceeds threshold 4)
                                    System.Console.WriteLine("nested");
                                }
                            } catch {}
                        }
                    }
                }
            }
        }
    "#;

    let file = file_facts("src/App.cs", "CSharp", content);
    let config = ScanConfig {
        max_control_flow_depth: 4,
        ..ScanConfig::default()
    };

    let findings = DeepControlFlowAudit.audit(&file, &config);

    assert_eq!(findings.len(), 1);
    assert_eq!(findings[0].evidence[0].line_start, 8); // line of `if (c) {`
    assert!(findings[0].evidence[0].snippet.contains("if (c)"));
}

// ── Kotlin ────────────────────────────────────────────────────────────────────

#[test]
fn kotlin_detects_exceeded_nesting_depth() {
    let content = r#"
        fun test() {
            if (a) { // depth 1
                for (x in y) { // depth 2
                    if (b) { // depth 3
                        when (z) { // depth 4
                            1 -> {
                                if (c) { // depth 5 (exceeds threshold 4)
                                    println("nested")
                                }
                            }
                        }
                    }
                }
            }
        }
    "#;

    let file = file_facts("src/App.kt", "Kotlin", content);
    let config = ScanConfig {
        max_control_flow_depth: 4,
        ..ScanConfig::default()
    };

    let findings = DeepControlFlowAudit.audit(&file, &config);

    assert_eq!(findings.len(), 1);
    assert_eq!(findings[0].evidence[0].line_start, 8); // line of `if (c) {`
    assert!(findings[0].evidence[0].snippet.contains("if (c)"));
}

#[test]
fn kotlin_flattens_else_if_chains() {
    let content = r#"
        fun test() {
            if (a) {
            } else if (b) {
            } else if (c) {
            }
        }
    "#;

    let file = file_facts("src/App.kt", "Kotlin", content);
    let config = ScanConfig {
        max_control_flow_depth: 2,
        ..ScanConfig::default()
    };

    let findings = DeepControlFlowAudit.audit(&file, &config);

    assert!(findings.is_empty(), "Kotlin else if chains should be flat");
}
