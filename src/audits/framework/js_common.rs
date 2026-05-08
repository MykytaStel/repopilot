use crate::audits::traits::ProjectAudit;
use crate::findings::types::{Evidence, Finding, FindingCategory, Severity};
use crate::scan::config::ScanConfig;
use crate::scan::facts::ScanFacts;

pub const JS_EXTENSIONS: &[&str] = &["ts", "tsx", "js", "jsx"];
const TEST_PATH_SEGMENTS: &[&str] = &[
    "test", "__tests__", "spec", "fixture", "fixtures", "mock", "mocks",
];

// ── VarDeclarationAudit ───────────────────────────────────────────────────────

pub struct VarDeclarationAudit;

impl ProjectAudit for VarDeclarationAudit {
    fn audit(&self, facts: &ScanFacts, _config: &ScanConfig) -> Vec<Finding> {
        let mut findings = Vec::new();

        for file in &facts.files {
            if !is_js_file(&file.path) || is_test_path(&file.path) {
                continue;
            }

            let content = match std::fs::read_to_string(&file.path) {
                Ok(c) => c,
                Err(_) => continue,
            };

            for (idx, line) in content.lines().enumerate() {
                let trimmed = line.trim();

                if is_comment_line(trimmed) {
                    continue;
                }

                if has_var_declaration(trimmed) {
                    findings.push(Finding {
                        id: String::new(),
                        rule_id: "framework.js.var-declaration".to_string(),
                        title: "var declaration found".to_string(),
                        description: concat!(
                            "`var` has function-level scope and is hoisted to the top of its function, ",
                            "which can cause subtle bugs when variables are accessed before assignment or escape block scope unexpectedly. ",
                            "Replace `var` with `const` (for values that do not change) or `let` (for values that do). ",
                            "Both are block-scoped and behave predictably."
                        ).to_string(),
                        category: FindingCategory::Framework,
                        severity: Severity::Low,
                        evidence: vec![Evidence {
                            path: file.path.clone(),
                            line_start: idx + 1,
                            line_end: None,
                            snippet: trimmed.to_string(),
                        }],
                    });
                    break;
                }
            }
        }

        findings
    }
}

// ── ConsoleLogAudit ───────────────────────────────────────────────────────────

pub struct ConsoleLogAudit;

impl ProjectAudit for ConsoleLogAudit {
    fn audit(&self, facts: &ScanFacts, _config: &ScanConfig) -> Vec<Finding> {
        let mut findings = Vec::new();

        for file in &facts.files {
            if !is_js_file(&file.path) || is_test_path(&file.path) {
                continue;
            }

            let content = match std::fs::read_to_string(&file.path) {
                Ok(c) => c,
                Err(_) => continue,
            };

            for (idx, line) in content.lines().enumerate() {
                let trimmed = line.trim();

                if is_comment_line(trimmed) {
                    continue;
                }

                if trimmed.contains("console.log(") {
                    findings.push(Finding {
                        id: String::new(),
                        rule_id: "framework.js.console-log".to_string(),
                        title: "console.log found in production source".to_string(),
                        description: concat!(
                            "`console.log` statements left in production code expose internal state and data to the device console, ",
                            "add unnecessary serialisation overhead, and are a minor security concern. ",
                            "Use a logging library that can be silenced in production builds ",
                            "(e.g. `react-native-logs` or `loglevel`), or wrap calls in `if (__DEV__)`."
                        ).to_string(),
                        category: FindingCategory::Framework,
                        severity: Severity::Low,
                        evidence: vec![Evidence {
                            path: file.path.clone(),
                            line_start: idx + 1,
                            line_end: None,
                            snippet: trimmed.to_string(),
                        }],
                    });
                    break;
                }
            }
        }

        findings
    }
}

// ── Shared helpers (pub so react_native.rs can reuse) ─────────────────────────

pub fn is_js_file(path: &std::path::Path) -> bool {
    path.extension()
        .and_then(|e| e.to_str())
        .map(|e| JS_EXTENSIONS.contains(&e))
        .unwrap_or(false)
}

pub fn is_test_path(path: &std::path::Path) -> bool {
    path.components().any(|c| {
        c.as_os_str()
            .to_str()
            .map(|s| TEST_PATH_SEGMENTS.iter().any(|seg| s == *seg))
            .unwrap_or(false)
    })
}

pub fn is_comment_line(trimmed: &str) -> bool {
    trimmed.starts_with("//")
        || trimmed.starts_with('*')
        || trimmed.starts_with("/*")
        || trimmed.starts_with("<!--")
}

fn has_var_declaration(trimmed: &str) -> bool {
    if trimmed.starts_with("var ") {
        return true;
    }
    // Token-based: split on whitespace/punctuation and check for exact "var" token.
    // This avoids false positives on identifiers like `typeVar` or `varName`.
    trimmed
        .split(|c: char| c.is_whitespace() || matches!(c, ';' | '(' | ',' | '{' | '}' | '='))
        .any(|token| token == "var")
}

// ── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use crate::scan::config::ScanConfig;
    use crate::scan::facts::{FileFacts, ScanFacts};
    use std::io::Write;
    use tempfile::tempdir;

    fn make_file_facts(path: std::path::PathBuf) -> FileFacts {
        FileFacts {
            path,
            language: Some("TypeScript".to_string()),
            lines_of_code: 2,
            branch_count: 0,
            imports: vec![],
            content: String::new(),
        }
    }

    // ── VarDeclarationAudit ───────────────────────────────────────────────────

    #[test]
    fn var_declaration_flagged() {
        let dir = tempdir().unwrap();
        let file_path = dir.path().join("utils.ts");
        write!(
            std::fs::File::create(&file_path).unwrap(),
            "var x = 1;\nconst y = 2;\n"
        )
        .unwrap();
        let mut facts = ScanFacts { root_path: dir.path().to_path_buf(), ..ScanFacts::default() };
        facts.files.push(make_file_facts(file_path));
        let findings = VarDeclarationAudit.audit(&facts, &ScanConfig::default());
        assert_eq!(findings.len(), 1);
        assert_eq!(findings[0].rule_id, "framework.js.var-declaration");
    }

    #[test]
    fn var_inside_identifier_not_flagged() {
        let dir = tempdir().unwrap();
        let file_path = dir.path().join("utils.ts");
        // "typeVar", "localStorage", "varName" should NOT trigger
        write!(
            std::fs::File::create(&file_path).unwrap(),
            "const typeVar = 1;\nconst localStorage = {{}};\nconst varName = 3;\n"
        )
        .unwrap();
        let mut facts = ScanFacts { root_path: dir.path().to_path_buf(), ..ScanFacts::default() };
        facts.files.push(make_file_facts(file_path));
        let findings = VarDeclarationAudit.audit(&facts, &ScanConfig::default());
        assert!(findings.is_empty(), "identifiers containing 'var' must not be flagged");
    }

    #[test]
    fn var_in_test_file_skipped() {
        let dir = tempdir().unwrap();
        let test_dir = dir.path().join("__tests__");
        std::fs::create_dir(&test_dir).unwrap();
        let file_path = test_dir.join("utils.test.ts");
        write!(std::fs::File::create(&file_path).unwrap(), "var x = 1;\n").unwrap();
        let mut facts = ScanFacts { root_path: dir.path().to_path_buf(), ..ScanFacts::default() };
        facts.files.push(make_file_facts(file_path));
        let findings = VarDeclarationAudit.audit(&facts, &ScanConfig::default());
        assert!(findings.is_empty());
    }

    #[test]
    fn no_var_no_finding() {
        let dir = tempdir().unwrap();
        let file_path = dir.path().join("utils.ts");
        write!(
            std::fs::File::create(&file_path).unwrap(),
            "const x = 1;\nlet y = 2;\n"
        )
        .unwrap();
        let mut facts = ScanFacts { root_path: dir.path().to_path_buf(), ..ScanFacts::default() };
        facts.files.push(make_file_facts(file_path));
        let findings = VarDeclarationAudit.audit(&facts, &ScanConfig::default());
        assert!(findings.is_empty());
    }

    // ── ConsoleLogAudit ───────────────────────────────────────────────────────

    #[test]
    fn console_log_flagged() {
        let dir = tempdir().unwrap();
        let file_path = dir.path().join("Screen.tsx");
        write!(
            std::fs::File::create(&file_path).unwrap(),
            "const x = 1;\nconsole.log(x);\n"
        )
        .unwrap();
        let mut facts = ScanFacts { root_path: dir.path().to_path_buf(), ..ScanFacts::default() };
        facts.files.push(FileFacts {
            path: file_path,
            language: Some("TypeScript React".to_string()),
            lines_of_code: 2,
            branch_count: 0,
            imports: vec![],
            content: String::new(),
        });
        let findings = ConsoleLogAudit.audit(&facts, &ScanConfig::default());
        assert_eq!(findings.len(), 1);
        assert_eq!(findings[0].rule_id, "framework.js.console-log");
    }

    #[test]
    fn console_log_in_comment_not_flagged() {
        let dir = tempdir().unwrap();
        let file_path = dir.path().join("utils.ts");
        write!(
            std::fs::File::create(&file_path).unwrap(),
            "// console.log(debug)\nconst x = 1;\n"
        )
        .unwrap();
        let mut facts = ScanFacts { root_path: dir.path().to_path_buf(), ..ScanFacts::default() };
        facts.files.push(make_file_facts(file_path));
        let findings = ConsoleLogAudit.audit(&facts, &ScanConfig::default());
        assert!(findings.is_empty());
    }
}
