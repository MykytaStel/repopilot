use crate::audits::code_quality::function_spans::for_each_function;
use crate::findings::types::Finding;
use std::path::Path;
use tree_sitter::Tree;

use super::LongFunctionPolicy;

/// Detects long functions from a parsed syntax tree.
///
/// Visits every function-like node (declarations, methods, named function
/// expressions, and arrow functions) via the shared `function_spans` walker and
/// flags those whose line span exceeds the policy threshold. Anonymous
/// functions use a doubled threshold to match the lower expectation for inline
/// callbacks.
pub(super) fn detect_ast(
    tree: &Tree,
    content: &str,
    language: &str,
    path: &Path,
    policy: LongFunctionPolicy,
) -> Vec<Finding> {
    let mut findings = Vec::new();
    for_each_function(tree, content, language, &mut |node, name, is_anonymous| {
        let start_row = node.start_position().row;
        let end_row = node.end_position().row;
        let fn_len = end_row.saturating_sub(start_row) + 1;

        // Inline callbacks are expected to be shorter; doubling the threshold
        // keeps them from dominating the signal, mirroring the prior heuristic.
        let threshold = if is_anonymous {
            policy.threshold.saturating_mul(2)
        } else {
            policy.threshold
        };

        if fn_len > threshold {
            let effective = LongFunctionPolicy {
                threshold,
                ..policy
            };
            findings.push(super::build_finding(
                path,
                start_row + 1,
                end_row + 1,
                name,
                fn_len,
                effective,
            ));
        }
    });
    findings
}
