//! Grammar-neutral helpers the per-language integrity recognizers share:
//! reading string literals, naming the enclosing test, and bounding text.

use tree_sitter::Node;

const MAX_TEXT_CHARS: usize = 80;

/// The text of a node with all whitespace removed (`it .only` → `it.only`).
pub(crate) fn compact_text(node: Node<'_>, content: &str) -> Option<String> {
    let text = node.utf8_text(content.as_bytes()).ok()?;
    Some(text.split_whitespace().collect())
}

/// Whether a node is a string literal in any supported grammar.
pub(crate) fn is_string_literal(node: Node<'_>) -> bool {
    matches!(
        node.kind(),
        "string"
            | "template_string"
            | "string_literal"
            | "raw_string_literal"
            | "interpreted_string_literal"
    )
}

/// The contents of a string literal node, without prefixes or quotes.
pub(crate) fn string_value(node: Node<'_>, content: &str) -> Option<String> {
    if !is_string_literal(node) {
        return None;
    }
    let text = node.utf8_text(content.as_bytes()).ok()?;
    Some(bounded(unquote(text)))
}

/// Strips string prefixes (`r`, `f`, `b`, `u`, raw-string hashes) and one
/// layer of matching quotes, including Python triple quotes.
pub(crate) fn unquote(text: &str) -> &str {
    let text = text
        .trim()
        .trim_start_matches(['r', 'R', 'b', 'B', 'u', 'U', 'f', 'F'])
        .trim_matches('#');
    for quote in ["\"\"\"", "'''", "\"", "'", "`"] {
        if let Some(inner) = text
            .strip_prefix(quote)
            .and_then(|rest| rest.strip_suffix(quote))
        {
            return inner;
        }
    }
    text
}

/// The first argument of a call when it is a string literal.
pub(crate) fn first_argument_string(call: Node<'_>, content: &str) -> Option<String> {
    let arguments = call.child_by_field_name("arguments")?;
    let first = arguments.named_child(0)?;
    string_value(first, content)
}

/// The first string literal among a call's arguments, looking one level into
/// keyword arguments (`reason="flaky"`).
pub(crate) fn any_argument_string(arguments: Node<'_>, content: &str) -> Option<String> {
    let mut cursor = arguments.walk();
    for argument in arguments.named_children(&mut cursor) {
        if let Some(value) = string_value(argument, content) {
            return Some(value);
        }
        if argument.kind() == "keyword_argument"
            && let Some(value) = argument
                .child_by_field_name("value")
                .and_then(|value| string_value(value, content))
        {
            return Some(value);
        }
    }
    None
}

/// The name of the nearest enclosing function declaration of one of `kinds`.
pub(crate) fn enclosing_function_name(
    node: Node<'_>,
    content: &str,
    kinds: &[&str],
) -> Option<String> {
    let mut current = node.parent();
    while let Some(candidate) = current {
        if kinds.contains(&candidate.kind()) {
            return candidate
                .child_by_field_name("name")
                .and_then(|name| name.utf8_text(content.as_bytes()).ok())
                .map(bounded);
        }
        current = candidate.parent();
    }
    None
}

/// Whether a node is a constant literal in any supported grammar.
pub(crate) fn is_literal(node: Node<'_>) -> bool {
    matches!(
        node.kind(),
        "true"
            | "false"
            | "null"
            | "undefined"
            | "none"
            | "nil"
            | "number"
            | "integer"
            | "float"
            | "int_literal"
            | "float_literal"
            | "integer_literal"
            | "boolean_literal"
    ) || (is_string_literal(node)
        && !has_named_child_of(node, &["template_substitution", "interpolation"]))
}

/// Whether every argument of a call is a literal — a comparison that cannot
/// observe the code under test (`assert.equal(1, 1)`, `assertTrue(True)`).
/// An empty argument list is not constant.
pub(crate) fn all_arguments_literal(arguments: Node<'_>) -> bool {
    let mut cursor = arguments.walk();
    let mut seen = false;
    for argument in arguments.named_children(&mut cursor) {
        let value = if argument.kind() == "keyword_argument" {
            match argument.child_by_field_name("value") {
                Some(value) => value,
                None => return false,
            }
        } else {
            argument
        };
        if matches!(value.kind(), "comment" | "line_comment" | "block_comment") {
            continue;
        }
        if !is_literal(value) {
            return false;
        }
        seen = true;
    }
    seen
}

fn has_named_child_of(node: Node<'_>, kinds: &[&str]) -> bool {
    let mut cursor = node.walk();
    node.named_children(&mut cursor)
        .any(|child| kinds.contains(&child.kind()))
}

/// A suppression directive recognized inside comment text.
pub(crate) struct Directive {
    pub(crate) text: &'static str,
    /// Whether rule names follow the directive (`noqa: E501`) and belong in
    /// the label, rather than free-text explanation (`@ts-ignore: legacy`).
    pub(crate) takes_rules: bool,
}

/// The suppression label a comment carries, if any. Directives are matched in
/// order, so list longer spellings first (`eslint-disable-next-line` before
/// `eslint-disable`).
pub(crate) fn comment_suppression(
    node: Node<'_>,
    content: &str,
    directives: &[Directive],
) -> Option<String> {
    let text = node.utf8_text(content.as_bytes()).ok()?;
    for directive in directives {
        let Some(start) = text.find(directive.text) else {
            continue;
        };
        if !directive.takes_rules {
            return Some(directive.text.to_string());
        }
        let rest = text[start + directive.text.len()..]
            .trim_end_matches("*/")
            .split(" -- ")
            .next()
            .unwrap_or_default()
            .trim();
        let separator = if rest.starts_with([':', '[', '=', '(']) {
            ""
        } else {
            " "
        };
        return Some(if rest.is_empty() {
            directive.text.to_string()
        } else {
            bounded(&format!("{}{separator}{rest}", directive.text))
        });
    }
    None
}

/// Truncates on a character boundary so reported text stays short.
pub(crate) fn bounded(text: &str) -> String {
    let text = text.trim().replace('\n', " ");
    if text.chars().count() <= MAX_TEXT_CHARS {
        text
    } else {
        let mut short: String = text.chars().take(MAX_TEXT_CHARS).collect();
        short.push('…');
        short
    }
}
