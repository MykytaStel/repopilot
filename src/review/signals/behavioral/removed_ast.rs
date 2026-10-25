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
use std::collections::BTreeMap;
use tree_sitter::{Node, Tree};

/// A changed handler identity scoped to the language frontend that recognized
/// it. Most forms use exact syntax tokens; Rust `match`/`?` Result propagation
/// uses the direct operation callee so extraction can preserve identity across
/// that syntax change. The containing function is excluded because extraction
/// commonly moves a handler into a new function.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub(crate) struct ErrorHandlerFingerprint {
    table_identity: usize,
    identity: ErrorHandlerIdentity,
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
enum ErrorHandlerIdentity {
    ExactTokens(Vec<(String, String)>),
    RustResultOperation(String),
}

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

/// Fingerprints error handlers whose start line belongs to the changed range.
/// Comments and formatting are ignored. Other than Rust `match`/`?` Result
/// propagation, identifiers, literals, and syntax tokens remain exact. Parse
/// failures are excluded so ambiguous moves remain visible as removals.
pub(crate) fn changed_error_handler_fingerprints(
    source: &ReviewSource,
    file: &ChangedFile,
    tables: &'static RemovedTables,
    is_pre: bool,
) -> BTreeMap<ErrorHandlerFingerprint, usize> {
    let Some(tree) = source.tree() else {
        return BTreeMap::new();
    };
    let root = tree.root_node();
    if root.has_error() {
        return BTreeMap::new();
    }
    let mut fingerprints = BTreeMap::new();
    let is_rust = file
        .path
        .extension()
        .and_then(|extension| extension.to_str())
        .is_some_and(|extension| extension.eq_ignore_ascii_case("rs"));
    walk_changed_error_handlers(
        root,
        source.content(),
        file,
        tables,
        is_rust,
        is_pre,
        &mut fingerprints,
    );
    fingerprints
}

fn walk_changed_error_handlers(
    node: Node<'_>,
    content: &str,
    file: &ChangedFile,
    tables: &'static RemovedTables,
    is_rust: bool,
    is_pre: bool,
    fingerprints: &mut BTreeMap<ErrorHandlerFingerprint, usize>,
) {
    let line = node.start_position().row + 1;
    let in_changed_range = if is_pre {
        file.hunks.iter().any(|hunk| {
            hunk.old_range
                .is_some_and(|range| line >= range.start && line <= range.end)
        })
    } else {
        file.contains_line(line)
    };

    if in_changed_range
        && ((tables.is_error_handling)(node, content)
            || (is_rust && is_rust_try_propagation(node, content)))
        && let Some(identity) = error_handler_identity(node, content, is_rust)
    {
        let fingerprint = ErrorHandlerFingerprint {
            table_identity: tables as *const RemovedTables as usize,
            identity,
        };
        *fingerprints.entry(fingerprint).or_default() += 1;
    }

    let mut cursor = node.walk();
    for child in node.children(&mut cursor) {
        walk_changed_error_handlers(child, content, file, tables, is_rust, is_pre, fingerprints);
    }
}

fn is_rust_try_propagation(node: Node<'_>, content: &str) -> bool {
    node.kind() == "try_expression"
        && node.named_child(0).is_some_and(|value| {
            callee_text(value, content)
                .is_none_or(|callee| !callee.contains(".map_err") && !callee.contains(".unwrap_or"))
        })
}

fn error_handler_identity(
    node: Node<'_>,
    content: &str,
    is_rust: bool,
) -> Option<ErrorHandlerIdentity> {
    if is_rust && matches!(node.kind(), "match_expression" | "try_expression") {
        let operation = if node.kind() == "match_expression" {
            node.child_by_field_name("value")
        } else {
            node.named_child(0)
        }?;
        if operation.kind() == "call_expression"
            && let Some(callee) = callee_text(operation, content)
        {
            return Some(ErrorHandlerIdentity::RustResultOperation(
                callee.trim().to_string(),
            ));
        }
    }

    token_fingerprint(node, content).map(ErrorHandlerIdentity::ExactTokens)
}

fn token_fingerprint(node: Node<'_>, content: &str) -> Option<Vec<(String, String)>> {
    fn collect(node: Node<'_>, content: &str, tokens: &mut Vec<(String, String)>) -> Option<()> {
        if node.is_extra() {
            return Some(());
        }
        if node.child_count() == 0 {
            let text = node.utf8_text(content.as_bytes()).ok()?;
            tokens.push((node.kind().to_string(), text.to_string()));
            return Some(());
        }

        let mut cursor = node.walk();
        for child in node.children(&mut cursor) {
            collect(child, content, tokens)?;
        }
        Some(())
    }

    let mut tokens = vec![(node.kind().to_string(), String::new())];
    collect(node, content, &mut tokens)?;
    (tokens.len() > 1).then_some(tokens)
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
