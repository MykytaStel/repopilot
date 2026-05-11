use crate::findings::types::Finding;
use std::path::Path;

pub(super) fn detect_brace_based(
    content: &str,
    language: &str,
    path: &Path,
    policy: super::LongFunctionPolicy,
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

        // Anonymous functions (callbacks, arrow funcs) get a 2× threshold to reduce noise
        let effective_threshold = if fn_name.is_empty() {
            policy.threshold.saturating_mul(2)
        } else {
            policy.threshold
        };

        let effective_policy = super::LongFunctionPolicy {
            threshold: effective_threshold,
            ..policy
        };
        if fn_len > effective_threshold {
            findings.push(super::build_finding(
                path,
                fn_start_line,
                fn_end + 1,
                &fn_name,
                fn_len,
                effective_policy,
            ));
        }

        i = fn_end + 1;
    }

    findings
}

fn kotlin_strip_modifiers(trimmed: &str) -> &str {
    let modifiers = [
        "private ",
        "public ",
        "protected ",
        "internal ",
        "open ",
        "override ",
        "abstract ",
        "suspend ",
        "inline ",
        "operator ",
        "external ",
        "tailrec ",
        "actual ",
        "expect ",
    ];
    let mut s = trimmed;
    loop {
        let before = s;
        for m in modifiers {
            s = s.strip_prefix(m).unwrap_or(s);
        }
        if s == before {
            break;
        }
    }
    s
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
        "CSharp" | "C#" | "CS" => {
            trimmed.contains('(')
                && (trimmed.ends_with(") {") || trimmed.ends_with("){"))
                && !trimmed.starts_with("if ")
                && !trimmed.starts_with("} else")
                && !trimmed.starts_with("else ")
                && !trimmed.starts_with("for ")
                && !trimmed.starts_with("foreach ")
                && !trimmed.starts_with("while ")
                && !trimmed.starts_with("switch ")
                && !trimmed.starts_with("try {")
                && !trimmed.starts_with("catch ")
                && !trimmed.starts_with("using ")
                && !trimmed.starts_with("lock ")
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
        "Java" => {
            trimmed.contains('(')
                && (trimmed.ends_with(") {") || trimmed.ends_with("){"))
                && !trimmed.starts_with("if ")
                && !trimmed.starts_with("} else")
                && !trimmed.starts_with("else ")
                && !trimmed.starts_with("for ")
                && !trimmed.starts_with("while ")
                && !trimmed.starts_with("switch ")
                && !trimmed.starts_with("try {")
                && !trimmed.starts_with("catch ")
                && !trimmed.starts_with("synchronized ")
        }
        "Kotlin" => {
            let s = kotlin_strip_modifiers(trimmed);
            s.starts_with("fun ") && trimmed.contains('(')
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
        "Kotlin" => {
            let s = kotlin_strip_modifiers(trimmed);
            if let Some(pos) = s.find("fun ") {
                let rest = &s[pos + 4..];
                let end = rest.find('(').unwrap_or(rest.len());
                rest[..end].trim().to_string()
            } else {
                String::new()
            }
        }
        _ => {
            // Generic: word immediately before the first `(` — covers Java and others
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
