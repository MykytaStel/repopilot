//! AST walks backing the "removed behavioral signal" detectors in
//! [`super::removed`]: counting test cases, try/catch (error-handling) blocks,
//! and auth-check calls within the changed ranges of a diff.

use super::keywords;
use crate::review::diff::ChangedFile;
use crate::review::signals::content::ReviewSource;
use tree_sitter::{Node, Tree};

/// The callee portion of a call node — its `object.method` path with the argument
/// list stripped off, so matching never sees arguments or string literals.
/// `getUserProfile(session.userId)` yields `getUserProfile`, not the whole call.
/// Falls back to the `function`/`name` field when there is no `arguments` child.
fn callee_text<'a>(node: Node<'_>, content: &'a str) -> Option<&'a str> {
    if let Some(args) = node.child_by_field_name("arguments") {
        return content
            .get(node.start_byte()..args.start_byte())
            .map(str::trim)
            .filter(|callee| !callee.is_empty());
    }
    node.child_by_field_name("function")
        .or_else(|| node.child_by_field_name("name"))
        .and_then(|callee| callee.utf8_text(content.as_bytes()).ok())
}

/// Counts the test cases in a source file, or `None` if it can't be parsed.
pub(super) fn count_test_cases(source: &ReviewSource, ext: &str) -> Option<usize> {
    let tree = source.tree()?;
    let content = source.content();
    let mut count = 0;
    walk_tests(tree.root_node(), content, ext, &mut count);
    Some(count)
}

fn walk_tests(node: Node<'_>, content: &str, ext: &str, count: &mut usize) {
    let kind = node.kind();
    let text = node.utf8_text(content.as_bytes()).unwrap_or("");

    match ext {
        "js" | "mjs" | "cjs" | "ts" | "mts" | "cts" | "tsx" | "jsx" => {
            if kind == "call_expression"
                && let Some(callee) = node.child_by_field_name("function")
                && let Ok(callee_text) = callee.utf8_text(content.as_bytes())
            {
                let val = callee_text.trim();
                if val == "test" || val == "it" || val == "describe" {
                    *count += 1;
                }
            }
        }
        "py" => {
            if kind == "function_definition"
                && let Some(name_node) = node.child_by_field_name("name")
                && let Ok(name_text) = name_node.utf8_text(content.as_bytes())
                && name_text.starts_with("test_")
            {
                *count += 1;
            }
        }
        "go" => {
            if kind == "function_declaration"
                && let Some(name_node) = node.child_by_field_name("name")
                && let Ok(name_text) = name_node.utf8_text(content.as_bytes())
                && name_text.starts_with("Test")
            {
                *count += 1;
            }
        }
        "rs" if kind == "function_item" && text.contains("#[test]") => {
            *count += 1;
        }
        "java" | "kt" | "kts"
            if (kind == "method_declaration" || kind == "function_declaration")
                && text.contains("@Test") =>
        {
            *count += 1;
        }
        "cs" if kind == "method_declaration"
            && (text.contains("[Test]")
                || text.contains("[TestMethod]")
                || text.contains("[Fact]")) =>
        {
            *count += 1;
        }
        _ => {}
    }

    let mut cursor = node.walk();
    for child in node.children(&mut cursor) {
        walk_tests(child, content, ext, count);
    }
}

/// Counts try/catch (error-handling) blocks within the diff's changed ranges.
pub(super) fn count_try_blocks(
    tree: &Tree,
    content: &str,
    file: &ChangedFile,
    ext: &str,
    is_pre: bool,
) -> usize {
    let mut count = 0;
    walk_try(tree.root_node(), content, file, ext, is_pre, &mut count);
    count
}

fn walk_try(
    node: Node<'_>,
    content: &str,
    file: &ChangedFile,
    ext: &str,
    is_pre: bool,
    count: &mut usize,
) {
    let line = node.start_position().row + 1;
    let inside = if is_pre {
        file.hunks.iter().any(|h| {
            if let Some(r) = h.old_range {
                line >= r.start && line <= r.end
            } else {
                false
            }
        })
    } else {
        file.contains_line(line)
    };

    if inside {
        let kind = node.kind();
        let text = node.utf8_text(content.as_bytes()).unwrap_or("");

        let is_try = match ext {
            "js" | "mjs" | "cjs" | "ts" | "mts" | "cts" | "tsx" | "jsx" => kind == "try_statement",
            "py" => kind == "try_statement",
            "java" | "cs" => kind == "try_statement",
            "kt" | "kts" => kind == "try_expression",
            // Inspect only the `if` condition, not the whole statement body, so a
            // nested `if err != nil` does not inflate the enclosing block's count.
            "go" => {
                kind == "if_statement"
                    && node
                        .child_by_field_name("condition")
                        .and_then(|cond| cond.utf8_text(content.as_bytes()).ok())
                        .is_some_and(|cond| cond.contains("err") && cond.contains("nil"))
            }
            "rs" => {
                (kind == "match_expression" && text.contains("Err("))
                    || (kind == "if_let_expression" && text.contains("Err("))
                    // Match on the callee (`x.map_err`), not the whole call, so a
                    // `.unwrap_or` buried in an argument does not double-count.
                    || (kind == "call_expression"
                        && callee_text(node, content).is_some_and(|callee| {
                            callee.contains(".unwrap_or") || callee.contains(".map_err")
                        }))
            }
            _ => false,
        };
        if is_try {
            *count += 1;
        }
    }

    let mut cursor = node.walk();
    for child in node.children(&mut cursor) {
        walk_try(child, content, file, ext, is_pre, count);
    }
}

/// Counts auth-check calls (auth/login/jwt/permission/session/role) within the
/// diff's changed ranges.
pub(super) fn count_auth_checks(
    tree: &Tree,
    content: &str,
    file: &ChangedFile,
    ext: &str,
    is_pre: bool,
) -> usize {
    let mut count = 0;
    walk_auth(tree.root_node(), content, file, ext, is_pre, &mut count);
    count
}

fn walk_auth(
    node: Node<'_>,
    content: &str,
    file: &ChangedFile,
    ext: &str,
    is_pre: bool,
    count: &mut usize,
) {
    let line = node.start_position().row + 1;
    let inside = if is_pre {
        file.hunks.iter().any(|h| {
            if let Some(r) = h.old_range {
                line >= r.start && line <= r.end
            } else {
                false
            }
        })
    } else {
        file.contains_line(line)
    };

    if inside {
        let kind = node.kind();
        let is_call = match ext {
            "js" | "mjs" | "cjs" | "ts" | "mts" | "cts" | "tsx" | "jsx" => {
                kind == "call_expression"
            }
            "py" => kind == "call",
            "go" => kind == "call_expression",
            "rs" => kind == "call_expression",
            "java" | "kt" | "kts" => kind == "method_invocation" || kind == "call_expression",
            "cs" => kind == "invocation_expression",
            _ => false,
        };
        // Match only the callee (its `object.method` path), never the arguments
        // or string literals, and count each call node once — so
        // `getUserProfile(session.userId)` and `log("authenticate")` do not fire,
        // and nested calls cannot inflate the count.
        if is_call
            && let Some(callee) = callee_text(node, content)
            && keywords::callee_is_auth_check(callee)
        {
            *count += 1;
        }
    }

    let mut cursor = node.walk();
    for child in node.children(&mut cursor) {
        walk_auth(child, content, file, ext, is_pre, count);
    }
}
