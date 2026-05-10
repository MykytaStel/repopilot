use std::collections::HashSet;

pub(super) fn extract(content: &str) -> HashSet<String> {
    let mut result = HashSet::new();
    let mut in_block_comment = false;
    let mut pending: Option<String> = None;

    for line in content.lines() {
        let trimmed = line.trim();

        if in_block_comment {
            if trimmed.contains("*/") {
                in_block_comment = false;
            }
            continue;
        }
        if trimmed.starts_with("/*") {
            if !trimmed.contains("*/") {
                in_block_comment = true;
            }
            continue;
        }
        if trimmed.starts_with("//") || trimmed.starts_with('*') {
            continue;
        }

        if let Some(acc) = pending.take() {
            let combined = acc + " " + trimmed;
            if combined.contains(';') {
                for import in rust_use_imports(&combined) {
                    result.insert(import);
                }
            } else {
                pending = Some(combined);
            }
            continue;
        }

        let effective = strip_rust_visibility(trimmed);

        if effective.strip_prefix("use ").is_some() {
            if effective.contains(';') {
                for import in rust_use_imports(effective) {
                    result.insert(import);
                }
            } else {
                pending = Some(effective.to_string());
            }
        } else if let Some(rest) = effective.strip_prefix("mod ") {
            let rest = rest.trim();
            if rest.ends_with(';') {
                let name = rest.trim_end_matches(';').trim();
                if !name.is_empty() && !name.contains('{') && !name.contains(' ') {
                    result.insert(format!("mod::{name}"));
                }
            }
        }
    }

    result
}

fn strip_rust_visibility(s: &str) -> &str {
    if let Some(rest) = s.strip_prefix("pub(")
        && let Some(close) = rest.find(')')
    {
        return rest[close + 1..].trim_start();
    }
    s.strip_prefix("pub ").unwrap_or(s)
}

fn rust_use_imports(stmt: &str) -> Vec<String> {
    let stmt = stmt.trim();
    let body = stmt.strip_prefix("use ").unwrap_or(stmt);
    let body = body.trim_end_matches(';').trim();

    if let Some(brace_pos) = body.find('{') {
        let prefix = body[..brace_pos].trim_end_matches(':');
        let after = &body[brace_pos + 1..];
        let inner = after.trim_end_matches('}').trim();
        return inner
            .split(',')
            .filter_map(|item| {
                let item = item.trim();
                if item.is_empty() || item == "_" || item == "self" {
                    return None;
                }
                let path = item.split(" as ").next().unwrap_or(item).trim();
                if path.is_empty() {
                    return None;
                }
                if prefix.is_empty() {
                    Some(path.to_string())
                } else {
                    Some(format!("{prefix}::{path}"))
                }
            })
            .collect();
    }

    let path = body.split(" as ").next().unwrap_or(body).trim();
    if path.is_empty() {
        vec![]
    } else {
        vec![path.to_string()]
    }
}
