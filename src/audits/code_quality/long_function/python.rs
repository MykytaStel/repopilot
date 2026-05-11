use crate::findings::types::Finding;
use std::path::Path;

pub(super) fn detect_python(
    content: &str,
    path: &Path,
    policy: super::LongFunctionPolicy,
) -> Vec<Finding> {
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
                if fn_len > policy.threshold {
                    findings.push(super::build_finding(
                        path,
                        start,
                        line_num - 1,
                        name,
                        fn_len,
                        policy,
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
        if fn_len > policy.threshold {
            findings.push(super::build_finding(
                path, start, total, &name, fn_len, policy,
            ));
        }
    }

    findings
}
