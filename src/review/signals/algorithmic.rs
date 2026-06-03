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
use crate::scan::language::detect_language;
use lang::{function_name, is_control_flow_node, is_else_if, is_loop_node};
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

    if crate::audits::context::classify::helpers::is_test_file(&file.path, false) {
        return signals;
    }

    let Some(post) = post_source else {
        return signals;
    };
    let Some(language) = detect_language(&file.path) else {
        return signals;
    };

    let post_parsed = post.parsed();
    let Some(post_tree) = post_parsed.tree() else {
        return signals;
    };
    let post_fns = collect_functions(post_tree.root_node(), post.content(), language);

    let pre_fns: Vec<FnMetrics> = pre_source
        .map(|src| {
            let parsed = src.parsed();
            parsed
                .tree()
                .map(|tree| collect_functions(tree.root_node(), src.content(), language))
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

fn collect_functions(root: Node<'_>, content: &str, language: &str) -> Vec<FnMetrics> {
    let mut out = Vec::new();
    collect_visit(root, content, language, &mut out);
    out
}

fn collect_visit(node: Node<'_>, content: &str, language: &str, out: &mut Vec<FnMetrics>) {
    if let Some(name) = function_name(node, language, content) {
        let start_line = node.start_position().row + 1;
        let end_line = node.end_position().row + 1;
        let fn_text = node.utf8_text(content.as_bytes()).unwrap_or("");
        out.push(FnMetrics {
            max_nesting: subtree_max_nesting(node, language, 0),
            has_nested_loop: subtree_has_nested_loop(node, language, false),
            line_span: end_line.saturating_sub(start_line) + 1,
            is_recursive: calls_name(fn_text, &name) >= 2,
            name,
            start_line,
            end_line,
        });
    }

    let mut cursor = node.walk();
    for child in node.children(&mut cursor) {
        collect_visit(child, content, language, out);
    }
}

/// Deepest control-flow nesting under `node`. `if/else if` chains count once.
fn subtree_max_nesting(node: Node<'_>, language: &str, depth: usize) -> usize {
    let is_cf = is_control_flow_node(node.kind(), language) && !is_else_if(node, language);
    let here = if is_cf { depth + 1 } else { depth };
    let mut max = here;
    let mut cursor = node.walk();
    for child in node.children(&mut cursor) {
        max = max.max(subtree_max_nesting(child, language, here));
    }
    max
}

/// Whether any loop under `node` sits inside another loop (a potential O(n^2)).
fn subtree_has_nested_loop(node: Node<'_>, language: &str, inside_loop: bool) -> bool {
    let is_loop = is_loop_node(node.kind(), language);
    if is_loop && inside_loop {
        return true;
    }
    let next = inside_loop || is_loop;
    let mut cursor = node.walk();
    for child in node.children(&mut cursor) {
        if subtree_has_nested_loop(child, language, next) {
            return true;
        }
    }
    false
}

/// Counts word-boundaried `name(` call sites in `text`. The function's own
/// declaration is one such site, so a count `>= 2` means it calls itself.
fn calls_name(text: &str, name: &str) -> usize {
    if name.is_empty() {
        return 0;
    }
    let mut count = 0;
    let mut search_start = 0;
    while let Some(rel) = text[search_start..].find(name) {
        let start = search_start + rel;
        let end = start + name.len();
        let before_ok = start == 0 || !text[..start].chars().next_back().is_some_and(is_ident_char);
        let after_ok = text[end..].trim_start().starts_with('(');
        if before_ok && after_ok {
            count += 1;
        }
        search_start = end;
    }
    count
}

fn is_ident_char(c: char) -> bool {
    c.is_alphanumeric() || c == '_'
}
