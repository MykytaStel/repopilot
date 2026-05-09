use std::collections::HashSet;

/// Extracts raw import strings from file content based on language.
/// Returns a deduplicated list.
pub fn extract_imports(content: &str, language: Option<&str>) -> Vec<String> {
    let set: HashSet<String> = match language {
        Some("Rust") => extract_rust(content),
        Some("TypeScript")
        | Some("TypeScript React")
        | Some("JavaScript")
        | Some("JavaScript React") => extract_ts(content),
        Some("Python") => extract_python(content),
        Some("Go") => extract_go(content),
        Some("Java") => extract_java(content),
        Some("Kotlin") => extract_kotlin(content),
        _ => return Vec::new(),
    };
    set.into_iter().collect()
}

// ── Rust ─────────────────────────────────────────────────────────────────────

fn extract_rust(content: &str) -> HashSet<String> {
    let mut result = HashSet::new();
    let mut in_block_comment = false;
    let mut pending: Option<String> = None;

    for line in content.lines() {
        let trimmed = line.trim();

        // Block comment tracking
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

        // Continue accumulating multi-line use statement
        if let Some(acc) = pending.take() {
            let combined = acc + " " + trimmed;
            if combined.contains(';') {
                for imp in rust_use_imports(&combined) {
                    result.insert(imp);
                }
            } else {
                pending = Some(combined);
            }
            continue;
        }

        // Strip optional visibility modifier (pub, pub(crate), pub(super), …)
        let effective = strip_rust_visibility(trimmed);

        if let Some(rest) = effective.strip_prefix("use ") {
            if rest.contains(';') {
                for imp in rust_use_imports(effective) {
                    result.insert(imp);
                }
            } else {
                // Begin multi-line accumulation
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

/// Strips leading `pub`, `pub(crate)`, `pub(super)`, or `pub(in …)` from `s`.
fn strip_rust_visibility(s: &str) -> &str {
    if let Some(rest) = s.strip_prefix("pub(") {
        if let Some(close) = rest.find(')') {
            return rest[close + 1..].trim_start();
        }
    }
    s.strip_prefix("pub ").unwrap_or(s)
}

/// Parses a single `use …;` statement (possibly reconstructed from multiple
/// lines) and returns each imported path.
fn rust_use_imports(stmt: &str) -> Vec<String> {
    let stmt = stmt.trim();
    // Strip leading `use ` (after visibility stripping was already done)
    let body = stmt.strip_prefix("use ").unwrap_or(stmt);
    // Strip trailing `;`
    let body = body.trim_end_matches(';').trim();

    // Handle group imports:  crate::foo::{Bar, Baz}
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
                // Strip `as alias`
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

    // Simple import, possibly with `as alias`
    let path = body.split(" as ").next().unwrap_or(body).trim();
    if path.is_empty() {
        vec![]
    } else {
        vec![path.to_string()]
    }
}

// ── TypeScript / JavaScript ───────────────────────────────────────────────────

fn extract_ts(content: &str) -> HashSet<String> {
    let mut result = HashSet::new();

    for line in content.lines() {
        let trimmed = line.trim();

        if trimmed.starts_with("//") || trimmed.starts_with('*') || trimmed.starts_with("/*") {
            continue;
        }

        // import … from "…"  /  export … from "…"
        if (trimmed.starts_with("import ") || trimmed.starts_with("export "))
            && trimmed.contains(" from ")
        {
            if let Some(path) = extract_from_path(trimmed) {
                if is_relative(path) {
                    result.insert(path.to_string());
                }
            }
        }

        // require("…")
        if trimmed.contains("require(") {
            if let Some(path) = extract_require_path(trimmed) {
                if is_relative(path) {
                    result.insert(path.to_string());
                }
            }
        }
    }

    result
}

fn extract_from_path(line: &str) -> Option<&str> {
    let pos = line.rfind(" from ")?;
    let after = line[pos + 6..].trim();
    extract_string_literal(after)
}

fn extract_require_path(line: &str) -> Option<&str> {
    let pos = line.find("require(")?;
    let after = line[pos + 8..].trim();
    extract_string_literal(after)
}

fn extract_string_literal(s: &str) -> Option<&str> {
    if let Some(rest) = s.strip_prefix('"') {
        let end = rest.find('"')?;
        Some(&rest[..end])
    } else if let Some(rest) = s.strip_prefix('\'') {
        let end = rest.find('\'')?;
        Some(&rest[..end])
    } else if let Some(rest) = s.strip_prefix('`') {
        let end = rest.find('`')?;
        Some(&rest[..end])
    } else {
        None
    }
}

fn is_relative(path: &str) -> bool {
    path.starts_with('.') || path.starts_with('/')
}

// ── Python ────────────────────────────────────────────────────────────────────

fn extract_python(content: &str) -> HashSet<String> {
    let mut result = HashSet::new();

    for line in content.lines() {
        let trimmed = line.trim();

        if trimmed.starts_with('#') {
            continue;
        }

        // from <module> import <names>
        if let Some(rest) = trimmed.strip_prefix("from ") {
            if let Some(import_pos) = rest.find(" import ") {
                let module = rest[..import_pos].trim();
                if !module.is_empty() {
                    result.insert(module.to_string());
                }
            }
            continue;
        }

        // import <module> [as alias] [, <module2>]
        if let Some(rest) = trimmed.strip_prefix("import ") {
            for part in rest.split(',') {
                let module = part.split(" as ").next().unwrap_or(part).trim();
                if !module.is_empty() {
                    result.insert(module.to_string());
                }
            }
        }
    }

    result
}

// ── Go ────────────────────────────────────────────────────────────────────────

fn extract_go(content: &str) -> HashSet<String> {
    let mut result = HashSet::new();
    let mut in_import_block = false;

    for line in content.lines() {
        let trimmed = line.trim();

        if trimmed.starts_with("//") {
            continue;
        }

        if trimmed == "import (" {
            in_import_block = true;
            continue;
        }

        if in_import_block {
            if trimmed == ")" {
                in_import_block = false;
                continue;
            }
            if let Some(path) = extract_go_import_path(trimmed) {
                result.insert(path.to_string());
            }
            continue;
        }

        // Single-line: import "path"
        if let Some(rest) = trimmed.strip_prefix("import ") {
            if let Some(path) = extract_string_literal(rest.trim()) {
                result.insert(path.to_string());
            }
        }
    }

    result
}

/// Extracts the import path string from a line inside a Go `import (…)` block.
/// Handles: `"path"`, `alias "path"`, `_ "path"`.
fn extract_go_import_path(line: &str) -> Option<&str> {
    let start = line.find('"')?;
    let rest = &line[start + 1..];
    let end = rest.find('"')?;
    Some(&rest[..end])
}

// ── Java ──────────────────────────────────────────────────────────────────────

fn extract_java(content: &str) -> HashSet<String> {
    let mut result = HashSet::new();
    for line in content.lines() {
        let trimmed = line.trim();
        if trimmed.starts_with("//") || trimmed.starts_with('*') || trimmed.starts_with("/*") {
            continue;
        }
        // import com.example.Foo;  |  import static com.example.Foo.method;
        if let Some(rest) = trimmed.strip_prefix("import ") {
            let rest = rest
                .trim_start_matches("static ")
                .trim_end_matches(';')
                .trim();
            // Skip wildcard imports: com.example.*
            if !rest.is_empty() && !rest.ends_with('*') {
                result.insert(rest.to_string());
            }
        }
    }
    result
}

// ── Kotlin ────────────────────────────────────────────────────────────────────

fn extract_kotlin(content: &str) -> HashSet<String> {
    let mut result = HashSet::new();
    for line in content.lines() {
        let trimmed = line.trim();
        if trimmed.starts_with("//") || trimmed.starts_with("/*") || trimmed.starts_with('*') {
            continue;
        }
        if let Some(rest) = trimmed.strip_prefix("import ") {
            let rest = rest.trim_end_matches(';').trim();
            // Skip wildcard imports and destructuring aliases
            let base = rest.split(" as ").next().unwrap_or(rest).trim();
            if !base.is_empty() && !base.ends_with('*') {
                result.insert(base.to_string());
            }
        }
    }
    result
}
