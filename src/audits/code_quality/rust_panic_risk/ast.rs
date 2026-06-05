use super::pattern::RustPanicPattern;
use std::collections::HashMap;
use tree_sitter::Node;

/// Walks the parsed Rust tree and collects at most one panic-risk candidate per
/// line, keeping the highest-precedence pattern when several land on one line.
/// The returned nodes borrow `root`'s tree, so the caller must keep it alive.
pub(super) fn collect_candidates<'a>(
    root: Node<'a>,
    content: &str,
) -> HashMap<usize, (Node<'a>, RustPanicPattern)> {
    let mut candidates: HashMap<usize, (Node<'a>, RustPanicPattern)> = HashMap::new();
    visit(root, content, false, &mut candidates);
    candidates
}

fn visit<'a>(
    node: Node<'a>,
    content: &str,
    in_macro_args: bool,
    candidates: &mut HashMap<usize, (Node<'a>, RustPanicPattern)>,
) {
    let kind = node.kind();

    // Skip comments and string/char literals
    if matches!(
        kind,
        "line_comment" | "block_comment" | "string_literal" | "raw_string_literal" | "char_literal"
    ) {
        return;
    }

    if in_macro_args && (kind == "identifier" || kind == "field_identifier") {
        if let Ok(text) = node.utf8_text(content.as_bytes()) {
            let pattern = match text {
                "unwrap" => Some(RustPanicPattern::Unwrap),
                "unwrap_err" => Some(RustPanicPattern::UnwrapErr),
                "expect" => Some(RustPanicPattern::Expect),
                "expect_err" => Some(RustPanicPattern::ExpectErr),
                _ => None,
            };
            if let Some(pat) = pattern {
                if is_preceded_by_dot(node, content) && is_followed_by_paren(node, content) {
                    let line_number = node.start_position().row + 1;
                    let keep = if let Some((_, existing_pattern)) = candidates.get(&line_number) {
                        pat.precedence() > existing_pattern.precedence()
                    } else {
                        true
                    };
                    if keep {
                        candidates.insert(line_number, (node, pat));
                    }
                }
            }
        }
    }

    let is_panic_macro = if kind == "macro_invocation" {
        if let Some(pattern) = detect_ast_node(node, content) {
            let line_number = node.start_position().row + 1;
            let keep = if let Some((_, existing_pattern)) = candidates.get(&line_number) {
                pattern.precedence() > existing_pattern.precedence()
            } else {
                true
            };
            if keep {
                candidates.insert(line_number, (node, pattern));
            }
            true
        } else {
            false
        }
    } else {
        false
    };

    if !is_panic_macro {
        if let Some(pattern) = detect_ast_node(node, content) {
            let line_number = node.start_position().row + 1;
            let keep = if let Some((_, existing_pattern)) = candidates.get(&line_number) {
                pattern.precedence() > existing_pattern.precedence()
            } else {
                true
            };
            if keep {
                candidates.insert(line_number, (node, pattern));
            }
        }
    }

    let next_in_macro = in_macro_args || (kind == "macro_invocation" && !is_panic_macro);

    let mut cursor = node.walk();
    for child in node.children(&mut cursor) {
        visit(child, content, next_in_macro, candidates);
    }
}

fn detect_ast_node(node: Node<'_>, content: &str) -> Option<RustPanicPattern> {
    match node.kind() {
        "macro_invocation" => {
            let macro_node = node
                .child_by_field_name("macro")
                .or_else(|| node.child(0))?;
            let text = macro_node.utf8_text(content.as_bytes()).ok()?;
            let macro_name = text.split("::").last()?;
            match macro_name {
                "panic" => Some(RustPanicPattern::Panic),
                "todo" => Some(RustPanicPattern::Todo),
                "unimplemented" => Some(RustPanicPattern::Unimplemented),
                _ => None,
            }
        }
        "call_expression" => {
            let function = node.child_by_field_name("function")?;
            if function.kind() == "field_expression" {
                let field = function.child_by_field_name("field")?;
                let method_name = field.utf8_text(content.as_bytes()).ok()?;
                match method_name {
                    "unwrap" => Some(RustPanicPattern::Unwrap),
                    "unwrap_err" => Some(RustPanicPattern::UnwrapErr),
                    "expect" => Some(RustPanicPattern::Expect),
                    "expect_err" => Some(RustPanicPattern::ExpectErr),
                    _ => None,
                }
            } else {
                None
            }
        }
        _ => None,
    }
}

fn is_preceded_by_dot(node: Node<'_>, content: &str) -> bool {
    let start_byte = node.start_byte();
    if start_byte > 0 {
        let pre_slice = &content.as_bytes()[..start_byte];
        if let Some(last_char) = pre_slice.iter().rev().find(|&&b| !b.is_ascii_whitespace()) {
            return *last_char == b'.';
        }
    }
    false
}

fn is_followed_by_paren(node: Node<'_>, content: &str) -> bool {
    let end_byte = node.end_byte();
    if end_byte < content.len() {
        let post_slice = &content.as_bytes()[end_byte..];
        if let Some(first_char) = post_slice.iter().find(|&&b| !b.is_ascii_whitespace()) {
            return *first_char == b'(';
        }
    }
    false
}
