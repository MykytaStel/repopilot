use crate::audits::traits::ProjectAudit;
use crate::findings::types::{Evidence, Finding, FindingCategory, Severity};
use crate::knowledge::decision::apply_file_decision;
use crate::scan::config::ScanConfig;
use crate::scan::facts::ScanFacts;

pub const JS_EXTENSIONS: &[&str] = &["ts", "tsx", "js", "jsx"];
const TEST_PATH_SEGMENTS: &[&str] = &[
    "test",
    "__tests__",
    "spec",
    "fixture",
    "fixtures",
    "mock",
    "mocks",
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
                    let finding = Finding {
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
                        confidence: Default::default(),
                        evidence: vec![Evidence {
                            path: file.path.clone(),
                            line_start: idx + 1,
                            line_end: None,
                            snippet: trimmed.to_string(),
                        }],
                        workspace_package: None,
                        docs_url: None,
                    };
                    if let Some(finding) =
                        apply_file_decision("framework.js.var-declaration", file, finding, None)
                    {
                        findings.push(finding);
                    }
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
                    let finding = Finding {
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
                        confidence: Default::default(),
                        evidence: vec![Evidence {
                            path: file.path.clone(),
                            line_start: idx + 1,
                            line_end: None,
                            snippet: trimmed.to_string(),
                        }],
                        workspace_package: None,
                        docs_url: None,
                    };
                    if let Some(finding) =
                        apply_file_decision("framework.js.console-log", file, finding, None)
                    {
                        findings.push(finding);
                    }
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
            .map(|s| TEST_PATH_SEGMENTS.contains(&s))
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
mod tests;
