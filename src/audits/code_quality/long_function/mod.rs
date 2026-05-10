use crate::audits::traits::FileAudit;
use crate::findings::types::{Evidence, Finding, FindingCategory, Severity};
use crate::scan::config::ScanConfig;
use crate::scan::facts::FileFacts;
use std::path::Path;

mod brace;
mod python;

pub struct LongFunctionAudit;

impl FileAudit for LongFunctionAudit {
    fn audit(&self, file: &FileFacts, config: &ScanConfig) -> Vec<Finding> {
        if file.content.is_empty() {
            return vec![];
        }
        let language = match file.language.as_deref() {
            Some(l) if is_supported(l) => l,
            _ => return vec![],
        };
        detect_long_functions(
            &file.content,
            language,
            &file.path,
            config.long_function_loc_threshold,
        )
    }
}

fn is_supported(language: &str) -> bool {
    matches!(
        language,
        "Rust"
            | "Go"
            | "Python"
            | "TypeScript"
            | "TypeScript React"
            | "JavaScript"
            | "JavaScript React"
            | "Java"
            | "Kotlin"
    )
}

fn detect_long_functions(
    content: &str,
    language: &str,
    path: &Path,
    threshold: usize,
) -> Vec<Finding> {
    if language == "Python" {
        python::detect_python(content, path, threshold)
    } else {
        brace::detect_brace_based(content, language, path, threshold)
    }
}

fn build_finding(
    path: &Path,
    start_line: usize,
    end_line: usize,
    fn_name: &str,
    fn_len: usize,
    threshold: usize,
) -> Finding {
    let (title, name_display) = if fn_name.is_empty() {
        (
            "Long inline function".to_string(),
            "inline function".to_string(),
        )
    } else {
        (
            format!("Long function: `{fn_name}`"),
            format!("`{fn_name}`"),
        )
    };

    Finding {
        id: String::new(),
        rule_id: "code-quality.long-function".to_string(),
        title,
        description: format!(
            "Function {name_display} spans {fn_len} lines, exceeding the {threshold}-line threshold. \
             Long functions are harder to test and reason about — consider extracting helper functions."
        ),
        category: FindingCategory::CodeQuality,
        severity: Severity::Medium,
        evidence: vec![Evidence {
            path: path.to_path_buf(),
            line_start: start_line,
            line_end: Some(end_line),
            snippet: format!("function spans lines {start_line}–{end_line} ({fn_len} lines)"),
        }],
        workspace_package: None,
        docs_url: None,
    }
}
