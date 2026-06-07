//! Intra-procedural taint propagation.
//!
//! Two passes per function/module scope: seed a set of tainted local names from
//! assignments whose right-hand side reads a [`source`](super::sources) or
//! references an already-tainted name (iterated to a small fixpoint for chains),
//! then report each [`sink`](super::sinks) call — gated to the changed lines —
//! whose arguments carry a tainted value. Nested functions are analyzed with a
//! fresh map so local names do not leak between procedures.
//!
//! For SQL the report is suppressed unless the tainted value is built *into* the
//! query string (concatenation, interpolation, or a `format`/`Sprintf` call) or
//! is the query expression itself: a static query string with the value passed as
//! a separate bind parameter is the safe, parameterized pattern.

use super::ast::{callee_ends_with, callee_starts_with, first_named_arg, has_descendant_kind};
use super::sinks::{Sink, SinkKind, classify_sink};
use super::sources::{SourceKind, node_has_source};
use super::{TaintLang, TaintSignal};
use crate::review::diff::ChangedFile;
use crate::review::signals::behavioral::truncate_str;
use std::collections::HashMap;
use tree_sitter::Node;

pub(super) fn detect(
    root: Node<'_>,
    content: &str,
    lang: TaintLang,
    file: &ChangedFile,
    out: &mut Vec<TaintSignal>,
) {
    detect_scope(root, content, lang, file, out);
}

fn detect_scope(
    root: Node<'_>,
    content: &str,
    lang: TaintLang,
    file: &ChangedFile,
    out: &mut Vec<TaintSignal>,
) {
    let tainted = seed_tainted(root, content, lang);
    check_sinks(root, content, lang, file, &tainted, out);
    detect_nested_scopes(root, content, lang, file, out);
}

fn detect_nested_scopes(
    node: Node<'_>,
    content: &str,
    lang: TaintLang,
    file: &ChangedFile,
    out: &mut Vec<TaintSignal>,
) {
    let mut cursor = node.walk();
    for child in node.children(&mut cursor) {
        if lang.is_flow_scope(child) {
            detect_scope(child, content, lang, file, out);
        } else {
            detect_nested_scopes(child, content, lang, file, out);
        }
    }
}

// ── Seeding ─────────────────────────────────────────────────────────────────

fn seed_tainted(root: Node<'_>, content: &str, lang: TaintLang) -> HashMap<String, SourceKind> {
    let mut assignments: Vec<Assignment<'_>> = Vec::new();
    collect_assignments(root, lang, content, &mut assignments);
    // Process in document order so a later reassignment overrides an earlier one;
    // a single forward pass then resolves `a = src; b = a; c = b` chains.
    assignments.sort_by_key(|assignment| assignment.rhs.start_byte());

    let mut tainted: HashMap<String, SourceKind> = HashMap::new();
    for assignment in &assignments {
        match node_has_source(assignment.rhs, content, lang)
            .or_else(|| node_mentions_tainted(assignment.rhs, content, lang, &tainted))
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
    lang: TaintLang,
    content: &str,
    out: &mut Vec<Assignment<'a>>,
) {
    if let Some((lhs, rhs)) = assignment_parts(node, lang) {
        let mut names = Vec::new();
        collect_lhs_names(lhs, content, &mut names);
        if !names.is_empty() {
            out.push(Assignment {
                names,
                rhs,
                augmenting: is_augmenting(node, lang),
            });
        }
    }
    let mut cursor = node.walk();
    for child in node.children(&mut cursor) {
        if !lang.is_flow_scope(child) {
            collect_assignments(child, lang, content, out);
        }
    }
}

/// Whether `node` is a compound assignment that combines with the existing value
/// (`x += …`), as opposed to a plain replacement (`x = …`, `x := …`).
fn is_augmenting(node: Node<'_>, lang: TaintLang) -> bool {
    match lang {
        TaintLang::Python => node.kind() == "augmented_assignment",
        TaintLang::Go => {
            node.kind() == "assignment_statement" && {
                let mut cursor = node.walk();
                node.children(&mut cursor).any(|child| {
                    matches!(
                        child.kind(),
                        "+=" | "-="
                            | "*="
                            | "/="
                            | "%="
                            | "&="
                            | "|="
                            | "^="
                            | "<<="
                            | ">>="
                            | "&^="
                    )
                })
            }
        }
        // tree-sitter-javascript models `x += …` as a distinct
        // `augmented_assignment_expression` node that `assignment_parts` does not
        // collect, so anything collected here is a plain `=`.
        TaintLang::Js => false,
    }
}

/// The (target, value) nodes of an assignment-like node, per grammar.
fn assignment_parts<'a>(node: Node<'a>, lang: TaintLang) -> Option<(Node<'a>, Node<'a>)> {
    let kind = node.kind();
    let is_assignment = match lang {
        TaintLang::Js => matches!(kind, "variable_declarator" | "assignment_expression"),
        TaintLang::Python => matches!(kind, "assignment" | "augmented_assignment"),
        TaintLang::Go => matches!(kind, "short_var_declaration" | "assignment_statement"),
    };
    if !is_assignment {
        return None;
    }
    let (lhs_field, rhs_field) = if kind == "variable_declarator" {
        ("name", "value")
    } else {
        ("left", "right")
    };
    Some((
        node.child_by_field_name(lhs_field)?,
        node.child_by_field_name(rhs_field)?,
    ))
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
    lang: TaintLang,
    tainted: &HashMap<String, SourceKind>,
) -> Option<SourceKind> {
    if lang.is_flow_scope(node) {
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
        .find_map(|child| node_mentions_tainted(child, content, lang, tainted))
}

// ── Sink checking ─────────────────────────────────────────────────────────────

fn check_sinks(
    node: Node<'_>,
    content: &str,
    lang: TaintLang,
    file: &ChangedFile,
    tainted: &HashMap<String, SourceKind>,
    out: &mut Vec<TaintSignal>,
) {
    if let Some(sink) = classify_sink(node, content, lang) {
        let line = node.start_position().row + 1;
        if file.contains_line(line) {
            if let Some(source) = sink_taint(&sink, content, lang, tainted) {
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
    }

    let mut cursor = node.walk();
    for child in node.children(&mut cursor) {
        if !lang.is_flow_scope(child) {
            check_sinks(child, content, lang, file, tainted, out);
        }
    }
}

fn sink_taint(
    sink: &Sink<'_>,
    content: &str,
    lang: TaintLang,
    tainted: &HashMap<String, SourceKind>,
) -> Option<SourceKind> {
    match sink.kind {
        SinkKind::Sql => sql_taint(sink.args, content, lang, tainted),
        _ => node_has_source(sink.args, content, lang)
            .or_else(|| node_mentions_tainted(sink.args, content, lang, tainted)),
    }
}

/// SQL is unsafe only when the tainted value is part of the *query expression*
/// (first argument): built into the string, or a query variable/expression that
/// is itself tainted. A static query string with the value bound as a later
/// parameter argument is the safe pattern and yields `None`.
fn sql_taint(
    args: Node<'_>,
    content: &str,
    lang: TaintLang,
    tainted: &HashMap<String, SourceKind>,
) -> Option<SourceKind> {
    let first = first_named_arg(args)?;
    let first_text = first.utf8_text(content.as_bytes()).unwrap_or("");

    if is_string_building(first, content, lang) {
        return node_has_source(first, content, lang)
            .or_else(|| node_mentions_tainted(first, content, lang, tainted));
    }
    if first.kind() == "identifier" {
        return tainted.get(first_text).copied();
    }
    // A request value used directly as the whole query expression (rare, unsafe).
    node_has_source(first, content, lang)
}

/// Whether `node` constructs a string from parts — concatenation, interpolation,
/// or a `format`/`Sprintf` call — the positions where injected input is dangerous.
fn is_string_building(node: Node<'_>, content: &str, lang: TaintLang) -> bool {
    match lang {
        TaintLang::Js => matches!(node.kind(), "template_string" | "binary_expression"),
        TaintLang::Python => {
            node.kind() == "binary_operator"
                || (node.kind() == "string" && has_descendant_kind(node, "interpolation"))
                || (node.kind() == "call" && callee_ends_with(node, content, ".format"))
        }
        TaintLang::Go => {
            node.kind() == "binary_expression"
                || (node.kind() == "call_expression"
                    && callee_starts_with(node, content, "fmt.Sprint"))
        }
    }
}
