use crate::audits::traits::ProjectAudit;
use crate::findings::types::{Evidence, Finding, FindingCategory, Severity};
use crate::knowledge::decision::apply_file_decision;
use crate::scan::config::ScanConfig;
use crate::scan::facts::{FileContentProvider, ScanFacts};
use std::path::Path;

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
        audit_javascript_files(
            facts,
            has_var_declaration,
            build_var_finding,
            "framework.js.var-declaration",
        )
    }
}

// ── ConsoleLogAudit ───────────────────────────────────────────────────────────

pub struct ConsoleLogAudit;

impl ProjectAudit for ConsoleLogAudit {
    fn audit(&self, facts: &ScanFacts, _config: &ScanConfig) -> Vec<Finding> {
        audit_javascript_files(
            facts,
            |line| line.contains("console.log("),
            build_console_log_finding,
            "framework.js.console-log",
        )
    }
}

fn audit_javascript_files(
    facts: &ScanFacts,
    matches_line: fn(&str) -> bool,
    build_finding: fn(&Path, usize, &str) -> Finding,
    rule_id: &str,
) -> Vec<Finding> {
    let mut findings = Vec::new();
    for file in &facts.files {
        if !is_js_file(&file.path) || is_test_path(&file.path) {
            continue;
        }
        let Some(content) = FileContentProvider.content(file) else {
            continue;
        };
        let Some((line_number, snippet)) = content.lines().enumerate().find_map(|(idx, line)| {
            let trimmed = line.trim();
            (!is_comment_line(trimmed) && matches_line(trimmed)).then(|| (idx + 1, trimmed))
        }) else {
            continue;
        };
        let finding = build_finding(&file.path, line_number, snippet);
        if let Some(finding) = apply_file_decision(rule_id, file, finding, None) {
            findings.push(finding);
        }
    }
    findings
}

fn build_var_finding(path: &Path, line_start: usize, snippet: &str) -> Finding {
    build_finding(
        "framework.js.var-declaration",
        "var declaration found",
        concat!(
            "`var` has function-level scope and is hoisted to the top of its function, ",
            "which can cause subtle bugs when variables are accessed before assignment or escape block scope unexpectedly. ",
            "Replace `var` with `const` (for values that do not change) or `let` (for values that do). ",
            "Both are block-scoped and behave predictably."
        ),
        path,
        line_start,
        snippet,
    )
}

fn build_console_log_finding(path: &Path, line_start: usize, snippet: &str) -> Finding {
    build_finding(
        "framework.js.console-log",
        "console.log found in production source",
        concat!(
            "`console.log` statements left in production code expose internal state and data to the device console, ",
            "add unnecessary serialisation overhead, and are a minor security concern. ",
            "Use a logging library that can be silenced in production builds ",
            "(e.g. `react-native-logs` or `loglevel`), or wrap calls in `if (__DEV__)`."
        ),
        path,
        line_start,
        snippet,
    )
}

fn build_finding(
    rule_id: &str,
    title: &str,
    description: &str,
    path: &Path,
    line_start: usize,
    snippet: &str,
) -> Finding {
    Finding {
        id: String::new(),
        rule_id: rule_id.to_string(),
        recommendation: Finding::recommendation_for_rule_id(rule_id),
        title: title.to_string(),
        description: description.to_string(),
        category: FindingCategory::Framework,
        severity: Severity::Low,
        confidence: Default::default(),
        evidence: vec![Evidence {
            path: path.to_path_buf(),
            line_start,
            line_end: None,
            snippet: snippet.to_string(),
        }],
        workspace_package: None,
        docs_url: None,
        provenance: Default::default(),
        risk: Default::default(),
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
