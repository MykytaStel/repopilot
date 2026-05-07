use crate::audits::traits::FileAudit;
use crate::findings::types::{Evidence, Finding, FindingCategory, Severity};
use crate::scan::config::ScanConfig;
use crate::scan::facts::FileFacts;
use std::path::Path;

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
    )
}

fn detect_long_functions(
    content: &str,
    language: &str,
    path: &Path,
    threshold: usize,
) -> Vec<Finding> {
    if language == "Python" {
        detect_python(content, path, threshold)
    } else {
        detect_brace_based(content, language, path, threshold)
    }
}

// ── Brace-based detection (Rust, Go, TypeScript, JavaScript) ──────────────────

fn detect_brace_based(
    content: &str,
    language: &str,
    path: &Path,
    threshold: usize,
) -> Vec<Finding> {
    let mut findings = Vec::new();
    let lines: Vec<&str> = content.lines().collect();
    let mut i = 0;

    while i < lines.len() {
        let trimmed = lines[i].trim();

        if !is_function_start(trimmed, language) {
            i += 1;
            continue;
        }

        let fn_name = extract_name(trimmed, language);
        let fn_start_line = i + 1; // 1-based

        // Scan forward counting braces. `opened` becomes true on the first `{`.
        let mut depth: i32 = 0;
        let mut opened = false;
        let mut fn_end_idx: Option<usize> = None;

        'scan: for (j, &raw_line) in lines.iter().enumerate().skip(i) {
            let line = strip_line_comment(raw_line);
            for ch in line.chars() {
                match ch {
                    '{' => {
                        depth += 1;
                        opened = true;
                    }
                    '}' if opened => {
                        depth -= 1;
                        if depth == 0 {
                            fn_end_idx = Some(j);
                            break 'scan;
                        }
                    }
                    _ => {}
                }
            }
            // Give up waiting for the opening brace after 20 lines
            if !opened && j >= i + 20 {
                break;
            }
        }

        if !opened {
            i += 1;
            continue;
        }

        let fn_end = fn_end_idx.unwrap_or(lines.len().saturating_sub(1));
        let fn_len = fn_end.saturating_sub(i) + 1;

        if fn_len > threshold {
            findings.push(build_finding(
                path,
                fn_start_line,
                fn_end + 1,
                &fn_name,
                fn_len,
                threshold,
            ));
        }

        i = fn_end + 1;
    }

    findings
}

fn is_function_start(trimmed: &str, language: &str) -> bool {
    if trimmed.starts_with("//")
        || trimmed.starts_with('#')
        || trimmed.starts_with("/*")
        || trimmed.starts_with('*')
    {
        return false;
    }

    match language {
        "Rust" => {
            let s = trimmed
                .trim_start_matches("pub(crate) ")
                .trim_start_matches("pub(super) ")
                .trim_start_matches("pub ")
                .trim_start_matches("async ")
                .trim_start_matches("unsafe ")
                .trim_start_matches("const ")
                .trim_start_matches("extern ");
            s.starts_with("fn ") && trimmed.contains('(')
        }
        "Go" => trimmed.starts_with("func ") && trimmed.contains('('),
        "TypeScript" | "TypeScript React" | "JavaScript" | "JavaScript React" => {
            let has_fn_kw = trimmed.contains("function ") && trimmed.contains('(');
            // Method or arrow shorthand that opens a brace on the same line
            let is_method_like = trimmed.contains('(')
                && (trimmed.ends_with(") {")
                    || trimmed.ends_with("){")
                    || trimmed.ends_with(") => {"))
                && !trimmed.starts_with("if ")
                && !trimmed.starts_with("} else")
                && !trimmed.starts_with("else ")
                && !trimmed.starts_with("for ")
                && !trimmed.starts_with("while ")
                && !trimmed.starts_with("switch ")
                && !trimmed.starts_with("try {");
            has_fn_kw || is_method_like
        }
        _ => false,
    }
}

fn extract_name(trimmed: &str, language: &str) -> String {
    match language {
        "Rust" => {
            if let Some(pos) = trimmed.find("fn ") {
                let rest = &trimmed[pos + 3..];
                let end = rest
                    .find(|c: char| !c.is_alphanumeric() && c != '_')
                    .unwrap_or(rest.len());
                return rest[..end].to_string();
            }
            String::new()
        }
        "Go" => {
            let rest = trimmed.trim_start_matches("func").trim();
            // Skip optional receiver: "(Type) name("
            let rest = if rest.starts_with('(') {
                rest.find(')').map(|p| rest[p + 1..].trim()).unwrap_or(rest)
            } else {
                rest
            };
            let end = rest.find('(').unwrap_or(rest.len());
            rest[..end].trim().to_string()
        }
        _ => {
            // Generic: word immediately before the first `(`
            if let Some(paren) = trimmed.find('(') {
                trimmed[..paren]
                    .split_whitespace()
                    .last()
                    .unwrap_or("")
                    .to_string()
            } else {
                String::new()
            }
        }
    }
}

/// Strips `//` line comments. Intentionally simple — accepts occasional false
/// positives from `//` inside string literals as an acceptable heuristic cost.
fn strip_line_comment(line: &str) -> &str {
    line.find("//").map(|pos| &line[..pos]).unwrap_or(line)
}

// ── Python indentation-based detection ────────────────────────────────────────

fn detect_python(content: &str, path: &Path, threshold: usize) -> Vec<Finding> {
    let mut findings = Vec::new();
    let lines: Vec<&str> = content.lines().collect();
    let total = lines.len();

    // Stack of (start_line_1based, indent_len, fn_name)
    let mut stack: Vec<(usize, usize, String)> = Vec::new();

    for (i, line) in lines.iter().enumerate() {
        let stripped = line.trim_start();
        if stripped.is_empty() || stripped.starts_with('#') {
            continue;
        }
        let indent = line.len() - stripped.len();
        let line_num = i + 1;

        // Close any functions that have dedented to or past our level
        while let Some(&(_, fn_indent, _)) = stack.last() {
            if indent <= fn_indent {
                let Some((start, _, ref name)) = stack.pop() else {
                    break;
                };
                let fn_len = line_num - start;
                if fn_len > threshold {
                    findings.push(build_finding(
                        path,
                        start,
                        line_num - 1,
                        name,
                        fn_len,
                        threshold,
                    ));
                }
            } else {
                break;
            }
        }

        if stripped.starts_with("def ") && stripped.contains('(') {
            let rest = stripped.trim_start_matches("def ").trim();
            let end = rest.find('(').unwrap_or(rest.len());
            let fn_name = rest[..end].trim().to_string();
            stack.push((line_num, indent, fn_name));
        }
    }

    // Flush functions that extend to end of file
    for (start, _, name) in stack {
        let fn_len = total + 1 - start;
        if fn_len > threshold {
            findings.push(build_finding(path, start, total, &name, fn_len, threshold));
        }
    }

    findings
}

// ── Finding construction ──────────────────────────────────────────────────────

fn build_finding(
    path: &Path,
    start_line: usize,
    end_line: usize,
    fn_name: &str,
    fn_len: usize,
    threshold: usize,
) -> Finding {
    let name_display = if fn_name.is_empty() {
        "<anonymous>".to_string()
    } else {
        format!("`{fn_name}`")
    };

    Finding {
        id: String::new(),
        rule_id: "code-quality.long-function".to_string(),
        title: format!("Long function: {name_display}"),
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
    }
}
