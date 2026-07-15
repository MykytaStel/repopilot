//! AST walks backing the "removed behavioral signal" detectors in
//! [`super::removed`]: counting test cases, try/catch (error-handling) blocks,
//! and auth-check calls within the changed ranges of a diff. The per-language
//! recognizers live on the language frontends
//! ([`RemovedTables`](crate::review::signals::tables::RemovedTables),
//! extension-dispatched); a file whose extension no frontend claims counts
//! zero everywhere, exactly like the old fall-through arms.

use super::keywords;
use crate::languages::removed_for_extension;
use crate::review::diff::ChangedFile;
use crate::review::signals::content::ReviewSource;
use crate::review::signals::tables::RemovedTables;
use tree_sitter::{Node, Tree};

/// The callee portion of a call node — its `object.method` path with the argument
/// list stripped off, so matching never sees arguments or string literals.
/// `getUserProfile(session.userId)` yields `getUserProfile`, not the whole call.
/// Falls back to the `function`/`name` field when there is no `arguments` child.
pub(crate) fn callee_text<'a>(node: Node<'_>, content: &'a str) -> Option<&'a str> {
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
    if let Some(tables) = removed_for_extension(ext) {
        walk_tests(tree.root_node(), content, tables, &mut count);
    }
    Some(count)
}

fn walk_tests(node: Node<'_>, content: &str, tables: &'static RemovedTables, count: &mut usize) {
    if (tables.is_test_case)(node, content) {
        *count += 1;
    }

    let mut cursor = node.walk();
    for child in node.children(&mut cursor) {
        walk_tests(child, content, tables, count);
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
    let Some(tables) = removed_for_extension(ext) else {
        return 0;
    };
    let mut count = 0;
    walk_try(tree.root_node(), content, file, tables, is_pre, &mut count);
    count
}

fn walk_try(
    node: Node<'_>,
    content: &str,
    file: &ChangedFile,
    tables: &'static RemovedTables,
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

    if inside && (tables.is_error_handling)(node, content) {
        *count += 1;
    }

    let mut cursor = node.walk();
    for child in node.children(&mut cursor) {
        walk_try(child, content, file, tables, is_pre, count);
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
    let Some(tables) = removed_for_extension(ext) else {
        return 0;
    };
    let mut count = 0;
    walk_auth(tree.root_node(), content, file, tables, is_pre, &mut count);
    count
}

fn walk_auth(
    node: Node<'_>,
    content: &str,
    file: &ChangedFile,
    tables: &'static RemovedTables,
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

    // Match only the callee (its `object.method` path), never the arguments
    // or string literals, and count each call node once — so
    // `getUserProfile(session.userId)` and `log("authenticate")` do not fire,
    // and nested calls cannot inflate the count.
    if inside
        && tables.auth_call_kinds.contains(&node.kind())
        && let Some(callee) = callee_text(node, content)
        && keywords::callee_is_auth_check(callee)
    {
        *count += 1;
    }

    let mut cursor = node.walk();
    for child in node.children(&mut cursor) {
        walk_auth(child, content, file, tables, is_pre, count);
    }
}
