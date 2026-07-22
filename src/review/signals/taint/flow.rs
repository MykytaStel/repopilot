//! Intra-procedural taint propagation.
//!
//! Two passes per function/module scope: seed a set of tainted local names from
//! assignments whose right-hand side reads a [`source`](super::sources) or
//! references an already-tainted name — the assignments are processed in a single
//! document-order forward pass, so `a = src; b = a; c = b` chains carry through —
//! then report each [`sink`](super::sinks) call — gated to the changed lines —
//! whose arguments carry a tainted value. Nested functions are analyzed with a
//! fresh map so local names do not leak between procedures.
//!
//! For SQL the report is suppressed unless the tainted value is built *into* the
//! query string (concatenation, interpolation, or a `format`/`Sprintf` call) or
//! is the query expression itself: a static query string with the value passed as
//! a separate bind parameter is the safe, parameterized pattern.

use super::TaintSignal;
use super::ast::first_named_arg;
use super::sinks::{Sink, SinkKind};
use super::sources::{SourceKind, node_has_source};
use super::tables::TaintTables;
use crate::review::diff::ChangedFile;
use crate::review::signals::behavioral::truncate_str;
use std::collections::HashMap;
use tree_sitter::Node;

pub(super) fn detect(
    root: Node<'_>,
    content: &str,
    tables: &'static TaintTables,
    file: &ChangedFile,
    out: &mut Vec<TaintSignal>,
) {
    detect_scope(root, content, tables, file, out);
}

fn detect_scope(
    root: Node<'_>,
    content: &str,
    tables: &'static TaintTables,
    file: &ChangedFile,
    out: &mut Vec<TaintSignal>,
) {
    let tainted = seed_tainted(root, content, tables);
    check_sinks(root, content, tables, file, &tainted, out);
    detect_nested_scopes(root, content, tables, file, out);
}

fn detect_nested_scopes(
    node: Node<'_>,
    content: &str,
    tables: &'static TaintTables,
    file: &ChangedFile,
    out: &mut Vec<TaintSignal>,
) {
    let mut cursor = node.walk();
    for child in node.children(&mut cursor) {
        if (tables.is_flow_scope)(child) {
            detect_scope(child, content, tables, file, out);
        } else {
            detect_nested_scopes(child, content, tables, file, out);
        }
    }
}

// ── Seeding ─────────────────────────────────────────────────────────────────

fn seed_tainted(
    root: Node<'_>,
    content: &str,
    tables: &'static TaintTables,
) -> HashMap<String, SourceKind> {
    let mut assignments: Vec<Assignment<'_>> = Vec::new();
    collect_assignments(root, tables, content, &mut assignments);
    // Process in document order so a later reassignment overrides an earlier one;
    // a single forward pass then resolves `a = src; b = a; c = b` chains.
    assignments.sort_by_key(|assignment| assignment.rhs.start_byte());

    let mut tainted: HashMap<String, SourceKind> = HashMap::new();
    for assignment in &assignments {
        match node_has_source(assignment.rhs, content, tables)
            .or_else(|| node_mentions_tainted(assignment.rhs, content, tables, &tainted))
        {
            // A tainted/source value sets (or re-sets) taint for each target name.
            Some(kind) => {
                for name in &assignment.names {
                    tainted.insert(name.clone(), kind);
                }
            }
            // A clean reassignment clears taint — `x = req.query.id; x = "safe"`.
            // Compound assignment (`x += …`) combines with the old value, so it
            // never clears.
            None if !assignment.augmenting => {
                for name in &assignment.names {
                    tainted.remove(name);
                }
            }
            None => {}
        }
    }
    tainted
}

/// One assignment found in a scope: the local names it targets, its value node,
/// and whether it is a compound assignment (`+=`, …) that combines with the old
/// value rather than replacing it.
struct Assignment<'a> {
    names: Vec<String>,
    rhs: Node<'a>,
    augmenting: bool,
}

fn collect_assignments<'a>(
    node: Node<'a>,
    tables: &'static TaintTables,
    content: &str,
    out: &mut Vec<Assignment<'a>>,
) {
    if let Some((lhs, rhs)) = assignment_parts(node, tables) {
        let mut names = Vec::new();
        collect_lhs_names(lhs, content, &mut names);
        if !names.is_empty() {
            out.push(Assignment {
                names,
                rhs,
                augmenting: (tables.is_augmenting)(node),
            });
        }
    }
    let mut cursor = node.walk();
    for child in node.children(&mut cursor) {
        if !(tables.is_flow_scope)(child) {
            collect_assignments(child, tables, content, out);
        }
    }
}

/// The (target, value) nodes of an assignment-like node, per the grammar's
/// assignment node kinds from the language's [`TaintTables`].
fn assignment_parts<'a>(
    node: Node<'a>,
    tables: &'static TaintTables,
) -> Option<(Node<'a>, Node<'a>)> {
    let kind = node.kind();
    if !tables.assignment_kinds.contains(&kind) {
        return None;
    }
    let (lhs_field, rhs_field) = if kind == "variable_declarator" {
        ("name", "value")
    } else {
        ("left", "right")
    };
    let lhs = node.child_by_field_name(lhs_field)?;
    if let Some(rhs) = node.child_by_field_name(rhs_field) {
        return Some((lhs, rhs));
    }
    // C#'s grammar attaches a declarator's initializer as a trailing
    // anonymous child rather than a `value` field; a declarator without an
    // initializer (`let x;`, `int x;`) has no such child and stays `None`.
    if kind == "variable_declarator" {
        let mut cursor = node.walk();
        let init = node
            .named_children(&mut cursor)
            .filter(|child| child.id() != lhs.id())
            .last()?;
        return Some((lhs, init));
    }
    None
}

/// Simple local names bound by an assignment target. Member/index/selector
/// targets (`obj.field = …`, `m[k] = …`) bind no local name and are skipped.
fn collect_lhs_names(lhs: Node<'_>, content: &str, out: &mut Vec<String>) {
    match lhs.kind() {
        "identifier" | "shorthand_property_identifier_pattern" => {
            if let Ok(text) = lhs.utf8_text(content.as_bytes()) {
                out.push(text.to_string());
            }
        }
        "object_pattern"
        | "array_pattern"
        | "object_assignment_pattern"
        | "pattern_list"
        | "tuple_pattern"
        | "list_pattern"
        | "expression_list" => {
            let mut cursor = lhs.walk();
            for child in lhs.named_children(&mut cursor) {
                collect_lhs_names(child, content, out);
            }
        }
        "pair_pattern" => {
            if let Some(value) = lhs.child_by_field_name("value") {
                collect_lhs_names(value, content, out);
            }
        }
        _ => {}
    }
}

fn node_mentions_tainted(
    node: Node<'_>,
    content: &str,
    tables: &'static TaintTables,
    tainted: &HashMap<String, SourceKind>,
) -> Option<SourceKind> {
    if (tables.is_flow_scope)(node) {
        return None;
    }
    // A sanitizer/coercion call neutralizes whatever it wraps; do not descend.
    if super::sanitizers::is_sanitizer_call(node, content, tables) {
        return None;
    }
    if node.kind() == "identifier" {
        let text = node.utf8_text(content.as_bytes()).ok()?;
        if let Some(source) = tainted.get(text) {
            return Some(*source);
        }
    }

    let mut cursor = node.walk();
    node.named_children(&mut cursor)
        .find_map(|child| node_mentions_tainted(child, content, tables, tainted))
}

// ── Sink checking ─────────────────────────────────────────────────────────────

fn check_sinks(
    node: Node<'_>,
    content: &str,
    tables: &'static TaintTables,
    file: &ChangedFile,
    tainted: &HashMap<String, SourceKind>,
    out: &mut Vec<TaintSignal>,
) {
    if let Some(sink) = (tables.classify_sink)(node, content) {
        let line = node.start_position().row + 1;
        if file.contains_line(line)
            && let Some(source) = sink_taint(&sink, content, tables, tainted)
        {
            let call_text = node.utf8_text(content.as_bytes()).unwrap_or("");
            out.push(TaintSignal {
                source,
                sink: sink.kind,
                path: file.path_string(),
                line,
                detail: format!(
                    "{} reaches {}: {}",
                    source.label(),
                    sink.kind.label(),
                    truncate_str(call_text, 60)
                ),
            });
        }
    }

    let mut cursor = node.walk();
    for child in node.children(&mut cursor) {
        if !(tables.is_flow_scope)(child) {
            check_sinks(child, content, tables, file, tainted, out);
        }
    }
}

fn sink_taint(
    sink: &Sink<'_>,
    content: &str,
    tables: &'static TaintTables,
    tainted: &HashMap<String, SourceKind>,
) -> Option<SourceKind> {
    match sink.kind {
        SinkKind::Sql => sql_taint(sink.args, content, tables, tainted),
        _ => node_has_source(sink.args, content, tables)
            .or_else(|| node_mentions_tainted(sink.args, content, tables, tainted)),
    }
}

/// SQL is unsafe only when the tainted value is part of the *query expression*
/// (first argument): built into the string, or a query variable/expression that
/// is itself tainted. A static query string with the value bound as a later
/// parameter argument is the safe pattern and yields `None`.
fn sql_taint(
    args: Node<'_>,
    content: &str,
    tables: &'static TaintTables,
    tainted: &HashMap<String, SourceKind>,
) -> Option<SourceKind> {
    let first = first_named_arg(args)?;
    let first_text = first.utf8_text(content.as_bytes()).unwrap_or("");

    if (tables.is_string_building)(first, content) {
        return node_has_source(first, content, tables)
            .or_else(|| node_mentions_tainted(first, content, tables, tainted));
    }
    if first.kind() == "identifier" {
        return tainted.get(first_text).copied();
    }
    // A request value used directly as the whole query expression (rare, unsafe).
    node_has_source(first, content, tables)
}
