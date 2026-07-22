//! Algorithmic change signals — function-level structural deltas.
//!
//! For each function the diff touched, compare its shape *before* and *after* the
//! change and report the structural delta: control-flow nesting got deeper, a
//! nested loop (a potential O(n^2)) appeared, the function grew past a size
//! threshold, or it became recursive.
//!
//! These are the highest-noise, most-arguable signals, so they stay deliberately
//! humble: they report the structural fact ("max nesting 2 → 4"), never a verdict
//! ("too complex"). Consistent with the "flag, don't prove" stance in `mod.rs`,
//! and gated to the `maybe` tier when surfaced. Ships at `preview`.

mod lang;

use crate::review::diff::ChangedFile;
use crate::review::signals::content::ReviewSource;
use crate::review::signals::tables::AlgorithmicKinds;
use crate::scan::language::detect_language;
use lang::{function_name, is_call_node, is_control_flow_node, is_else_if, is_loop_node};
use serde::Serialize;
use std::collections::HashMap;
use tree_sitter::Node;

/// Functions longer than this (post-change) are reported as having grown.
const FUNCTION_GREW_THRESHOLD: usize = 50;
/// Only report a nesting increase when the new depth is at least this — a 1 → 2
/// bump is rarely worth a reviewer's eye.
const COMPLEXITY_FLOOR: usize = 3;

/// The kind of algorithmic change observed in a touched function.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum AlgorithmicKind {
    ComplexityIncreased,
    NestedLoopIntroduced,
    FunctionGrew,
    RecursionIntroduced,
}

/// One algorithmic delta in a function the diff touched.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct AlgorithmicSignal {
    pub kind: AlgorithmicKind,
    pub path: String,
    pub line: usize,
    pub detail: String,
}

/// Structural shape of one function, computed from its syntax subtree.
struct FnMetrics {
    name: String,
    start_line: usize,
    end_line: usize,
    max_nesting: usize,
    has_nested_loop: bool,
    line_span: usize,
    is_recursive: bool,
}

/// Detect algorithmic deltas for the functions a change touched.
///
/// Compares each post-change function against its pre-change counterpart (matched
/// by name and occurrence order). A function with no counterpart is newly added,
/// so its loops/recursion are reported as "introduced". Test files are skipped —
/// algorithmic shape there is rarely the reviewer's concern.
pub fn detect_algorithmic(
    file: &ChangedFile,
    pre_source: Option<&ReviewSource>,
    post_source: Option<&ReviewSource>,
) -> Vec<AlgorithmicSignal> {
    let mut signals = Vec::new();

    if crate::audits::context::classify::helpers::is_test_file(&file.path) {
        return signals;
    }

    let Some(post) = post_source else {
        return signals;
    };
    let Some(kinds) = detect_language(&file.path)
        .and_then(crate::languages::review_for_label)
        .map(|tables| tables.algorithmic)
    else {
        return signals;
    };

    let Some(post_tree) = post.tree() else {
        return signals;
    };
    let post_fns = collect_functions(post_tree.root_node(), post.content(), kinds);

    let pre_fns: Vec<FnMetrics> = pre_source
        .map(|src| {
            src.tree()
                .map(|tree| collect_functions(tree.root_node(), src.content(), kinds))
                .unwrap_or_default()
        })
        .unwrap_or_default();

    let mut occurrence: HashMap<&str, usize> = HashMap::new();
    for post_fn in &post_fns {
        let idx = {
            let counter = occurrence.entry(post_fn.name.as_str()).or_insert(0);
            let current = *counter;
            *counter += 1;
            current
        };

        if !function_overlaps_change(post_fn, file) {
            continue;
        }

        let pre_fn = pre_fns
            .iter()
            .filter(|candidate| candidate.name == post_fn.name)
            .nth(idx);

        emit_signals(file, post_fn, pre_fn, &mut signals);
    }

    signals
}

fn function_overlaps_change(func: &FnMetrics, file: &ChangedFile) -> bool {
    (func.start_line..=func.end_line).any(|line| file.contains_line(line))
}

fn emit_signals(
    file: &ChangedFile,
    post: &FnMetrics,
    pre: Option<&FnMetrics>,
    signals: &mut Vec<AlgorithmicSignal>,
) {
    let path = file.path_string();
    let mut push = |kind, detail| {
        signals.push(AlgorithmicSignal {
            kind,
            path: path.clone(),
            line: post.start_line,
            detail,
        });
    };

    // A pre counterpart yields true deltas; without one the function is new, so
    // its loops/recursion/length are themselves the "introduced" facts.
    let (pre_nesting, pre_nested_loop, pre_recursive, pre_span) = match pre {
        Some(pre) => (
            pre.max_nesting,
            pre.has_nested_loop,
            pre.is_recursive,
            pre.line_span,
        ),
        None => (0, false, false, 0),
    };

    if post.max_nesting > pre_nesting && post.max_nesting >= COMPLEXITY_FLOOR {
        push(
            AlgorithmicKind::ComplexityIncreased,
            format!(
                "fn `{}` control-flow nesting {pre_nesting} → {}",
                post.name, post.max_nesting
            ),
        );
    }
    if post.has_nested_loop && !pre_nested_loop {
        push(
            AlgorithmicKind::NestedLoopIntroduced,
            format!(
                "fn `{}` introduces a nested loop (potential O(n^2))",
                post.name
            ),
        );
    }
    if post.is_recursive && !pre_recursive {
        push(
            AlgorithmicKind::RecursionIntroduced,
            format!("fn `{}` is now recursive", post.name),
        );
    }
    if post.line_span > FUNCTION_GREW_THRESHOLD && post.line_span > pre_span {
        push(
            AlgorithmicKind::FunctionGrew,
            format!("fn `{}` grew to {} lines", post.name, post.line_span),
        );
    }
}

fn collect_functions(
    root: Node<'_>,
    content: &str,
    kinds: &'static AlgorithmicKinds,
) -> Vec<FnMetrics> {
    let mut out = Vec::new();
    collect_visit(root, content, kinds, &mut out);
    out
}

fn collect_visit(
    node: Node<'_>,
    content: &str,
    kinds: &'static AlgorithmicKinds,
    out: &mut Vec<FnMetrics>,
) {
    if let Some(name) = function_name(node, kinds, content) {
        let start_line = node.start_position().row + 1;
        let end_line = node.end_position().row + 1;
        out.push(FnMetrics {
            max_nesting: subtree_max_nesting(node, kinds, 0),
            has_nested_loop: subtree_has_nested_loop(node, kinds, false),
            line_span: end_line.saturating_sub(start_line) + 1,
            is_recursive: subtree_calls_function(node, kinds, content, &name, true),
            name,
            start_line,
            end_line,
        });
    }

    let mut cursor = node.walk();
    for child in node.children(&mut cursor) {
        collect_visit(child, content, kinds, out);
    }
}

/// Deepest control-flow nesting under `node`. `if/else if` chains count once.
fn subtree_max_nesting(node: Node<'_>, kinds: &'static AlgorithmicKinds, depth: usize) -> usize {
    let is_cf = is_control_flow_node(node.kind(), kinds) && !is_else_if(node, kinds);
    let here = if is_cf { depth + 1 } else { depth };
    let mut max = here;
    let mut cursor = node.walk();
    for child in node.children(&mut cursor) {
        max = max.max(subtree_max_nesting(child, kinds, here));
    }
    max
}

/// Whether any loop under `node` sits inside another loop (a potential O(n^2)).
fn subtree_has_nested_loop(
    node: Node<'_>,
    kinds: &'static AlgorithmicKinds,
    inside_loop: bool,
) -> bool {
    let is_loop = is_loop_node(node.kind(), kinds);
    if is_loop && inside_loop {
        return true;
    }
    let next = inside_loop || is_loop;
    let mut cursor = node.walk();
    for child in node.children(&mut cursor) {
        if subtree_has_nested_loop(child, kinds, next) {
            return true;
        }
    }
    false
}

fn subtree_calls_function(
    node: Node<'_>,
    kinds: &'static AlgorithmicKinds,
    content: &str,
    function: &str,
    is_root: bool,
) -> bool {
    if !is_root && function_name(node, kinds, content).is_some() {
        return false;
    }
    if is_call_node(node, kinds)
        && call_target(node, content).is_some_and(|target| {
            target == function
                || target == format!("self.{function}")
                || target == format!("this.{function}")
                || target == format!("Self::{function}")
        })
    {
        return true;
    }
    let mut cursor = node.walk();
    node.children(&mut cursor)
        .any(|child| subtree_calls_function(child, kinds, content, function, false))
}

fn call_target<'a>(node: Node<'a>, content: &'a str) -> Option<&'a str> {
    let target = node
        .child_by_field_name("function")
        .or_else(|| node.child_by_field_name("name"))
        .or_else(|| node.named_child(0))?;
    target.utf8_text(content.as_bytes()).ok().map(str::trim)
}
