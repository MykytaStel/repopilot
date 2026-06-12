use std::collections::{BTreeMap, HashSet};
use tree_sitter::{Node, Tree};

pub(super) fn extract_spans(tree: &Tree, content: &str) -> BTreeMap<String, (usize, usize)> {
    let mut result = BTreeMap::new();
    visit(tree.root_node(), content, &mut result);
    result
}

pub(super) fn extract(tree: &Tree, content: &str) -> HashSet<String> {
    extract_spans(tree, content).into_keys().collect()
}

fn visit(node: Node<'_>, content: &str, result: &mut BTreeMap<String, (usize, usize)>) {
    // Imports gated by `#[cfg(test)]` (the ubiquitous inline `mod tests`, or a
    // `#[cfg(test)] use ...;`) are compiled out of release builds, so they are
    // not production dependencies. Skipping the whole subtree keeps them out of
    // the import graph, which otherwise reads `#[cfg(test)] mod tests;` as a
    // production file importing a test file (false `architecture.test-leak`)
    // and lets test-only `use` edges form phantom cycles.
    if is_test_gated(node, content) {
        return;
    }

    let span = (node.start_position().row + 1, node.end_position().row + 1);
    match node.kind() {
        "use_declaration" => {
            if let Ok(text) = node.utf8_text(content.as_bytes()) {
                for path in rust_use_imports(text) {
                    result.entry(path).or_insert(span);
                }
            }
        }
        "mod_item" => {
            // `#[path = "..."] mod x;` points the module at a file resolved
            // relative to the current file's directory, so the plain
            // `mod::name` resolution would miss it. The attribute is a sibling
            // node preceding the `mod_item`, so look there first.
            if let Some(path) = mod_path_attr(node, content) {
                result.entry(format!("relfile::{path}")).or_insert(span);
            } else if let Ok(text) = node.utf8_text(content.as_bytes())
                && let Some(module) = rust_mod_import(text)
            {
                result.entry(module).or_insert(span);
            }
        }
        "macro_invocation" => {
            // `include!("rel/path.rs")` textually pulls another file into this
            // module; treat it as an edge so the included file is not seen as
            // dead code. The path is relative to the current file's directory.
            if let Ok(text) = node.utf8_text(content.as_bytes())
                && let Some(path) = rust_include_path(text)
            {
                result.entry(format!("relfile::{path}")).or_insert(span);
            }
        }
        _ => {}
    }

    let mut cursor = node.walk();
    for child in node.children(&mut cursor) {
        visit(child, content, result);
    }
}

fn rust_mod_import(stmt: &str) -> Option<String> {
    let effective = strip_rust_visibility(strip_rust_outer_attributes(stmt));
    let rest = effective.strip_prefix("mod ")?;
    let name = rest.trim().trim_end_matches(';').trim();
    (!name.is_empty() && !name.contains('{') && !name.contains(' ')).then(|| format!("mod::{name}"))
}

/// Reads the `#[path = "..."]` value from the outer attributes preceding a
/// `mod` item. tree-sitter-rust emits outer attributes as sibling
/// `attribute_item` nodes that precede the item, so we walk back over them.
fn mod_path_attr(mod_item: Node<'_>, content: &str) -> Option<String> {
    let mut prev = mod_item.prev_sibling();
    while let Some(node) = prev {
        if node.kind() != "attribute_item" {
            break;
        }
        if let Ok(text) = node.utf8_text(content.as_bytes())
            && let Some(path) = rust_mod_path_attr(text)
        {
            return Some(path);
        }
        prev = node.prev_sibling();
    }
    None
}

/// Extracts the file path from a single `#[path = "..."]` attribute's text.
fn rust_mod_path_attr(stmt: &str) -> Option<String> {
    let mut s = stmt.trim_start();
    while let Some(rest) = s.strip_prefix("#[") {
        let close = rest.find(']')?;
        let attr = rest[..close].trim();
        if let Some(value) = attr.strip_prefix("path") {
            let value = value.trim_start().strip_prefix('=')?.trim();
            return first_string_literal(value);
        }
        s = rest[close + 1..].trim_start();
    }
    None
}

/// Extracts the included file path from an `include!("...")` macro invocation.
/// `include_str!`/`include_bytes!` are intentionally not matched: they embed
/// data, not module code, so they should not create dead-code edges.
fn rust_include_path(stmt: &str) -> Option<String> {
    let rest = stmt.trim_start().strip_prefix("include")?;
    let rest = rest.trim_start().strip_prefix('!')?;
    first_string_literal(rest)
}

/// Returns the contents of the first double-quoted string literal in `input`.
fn first_string_literal(input: &str) -> Option<String> {
    let open = input.find('"')?;
    let rest = &input[open + 1..];
    let close = rest.find('"')?;
    Some(rest[..close].to_string())
}

/// True when `node` is annotated with a `#[cfg(test)]` or `#[test]` outer
/// attribute. tree-sitter-rust emits outer attributes as `attribute_item`
/// siblings preceding the item, so walk back over the contiguous run of them.
fn is_test_gated(node: Node<'_>, content: &str) -> bool {
    let mut prev = node.prev_sibling();
    while let Some(sibling) = prev {
        if sibling.kind() != "attribute_item" {
            return false;
        }
        if let Ok(text) = sibling.utf8_text(content.as_bytes())
            && attr_is_test(text)
        {
            return true;
        }
        prev = sibling.prev_sibling();
    }
    false
}

/// Recognizes `#[cfg(test)]` and `#[test]` once whitespace is removed.
fn attr_is_test(attr: &str) -> bool {
    let compact: String = attr.chars().filter(|c| !c.is_whitespace()).collect();
    compact.contains("cfg(test)") || compact == "#[test]"
}

fn strip_rust_outer_attributes(mut s: &str) -> &str {
    loop {
        let trimmed = s.trim_start();
        let Some(rest) = trimmed.strip_prefix("#[") else {
            return trimmed;
        };

        let mut depth = 1usize;
        let mut end = None;
        for (index, ch) in rest.char_indices() {
            match ch {
                '[' => depth += 1,
                ']' => {
                    depth = depth.saturating_sub(1);
                    if depth == 0 {
                        end = Some(index + ch.len_utf8());
                        break;
                    }
                }
                _ => {}
            }
        }

        let Some(end) = end else {
            return trimmed;
        };
        s = &rest[end..];
    }
}

fn strip_rust_visibility(s: &str) -> &str {
    if let Some(rest) = s.strip_prefix("pub(")
        && let Some(close) = rest.find(')')
    {
        return rest[close + 1..].trim_start();
    }
    s.strip_prefix("pub ").unwrap_or(s)
}

fn rust_use_imports(stmt: &str) -> Vec<String> {
    let body = stmt
        .trim()
        .strip_prefix("use ")
        .unwrap_or(stmt)
        .trim()
        .trim_end_matches(';')
        .trim();
    let mut out = Vec::new();
    expand_use_tree("", body, &mut out);
    out
}

fn expand_use_tree(prefix: &str, item: &str, out: &mut Vec<String>) {
    let item = item.trim();
    if item.is_empty() || item == "_" || item == "self" {
        return;
    }

    if let Some(open) = find_top_level_char(item, '{')
        && let Some(close) = matching_brace(item, open)
    {
        let before = item[..open].trim().trim_end_matches("::");
        let inner = &item[open + 1..close];
        let nested_prefix = join_rust_path(prefix, before);
        for child in split_top_level_commas(inner) {
            expand_use_tree(&nested_prefix, child, out);
        }
        return;
    }

    let path = item.split(" as ").next().unwrap_or(item).trim();
    if path.is_empty() || path == "_" || path == "self" {
        return;
    }
    let path = join_rust_path(prefix, path);
    if !path.is_empty() {
        out.push(path);
    }
}

fn split_top_level_commas(input: &str) -> Vec<&str> {
    let mut parts = Vec::new();
    let mut depth = 0usize;
    let mut start = 0usize;

    for (index, ch) in input.char_indices() {
        match ch {
            '{' => depth += 1,
            '}' => depth = depth.saturating_sub(1),
            ',' if depth == 0 => {
                parts.push(&input[start..index]);
                start = index + 1;
            }
            _ => {}
        }
    }
    parts.push(&input[start..]);
    parts
}

fn find_top_level_char(input: &str, needle: char) -> Option<usize> {
    let mut depth = 0usize;
    for (index, ch) in input.char_indices() {
        match ch {
            '{' if ch == needle && depth == 0 => return Some(index),
            '{' => depth += 1,
            '}' => depth = depth.saturating_sub(1),
            _ if ch == needle && depth == 0 => return Some(index),
            _ => {}
        }
    }
    None
}

fn matching_brace(input: &str, open: usize) -> Option<usize> {
    let mut depth = 0usize;
    for (index, ch) in input[open..].char_indices() {
        match ch {
            '{' => depth += 1,
            '}' => {
                depth = depth.saturating_sub(1);
                if depth == 0 {
                    return Some(open + index);
                }
            }
            _ => {}
        }
    }
    None
}

fn join_rust_path(prefix: &str, item: &str) -> String {
    let item = item.trim().trim_matches(':');
    if prefix.is_empty() {
        item.to_string()
    } else if item.is_empty() {
        prefix.trim_end_matches("::").to_string()
    } else {
        format!("{}::{}", prefix.trim_end_matches("::"), item)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::analysis::parse::{ParseLanguage, parse};

    fn imports(src: &str) -> HashSet<String> {
        let tree = parse(src, ParseLanguage::Rust).expect("rust parses");
        extract(&tree, src)
    }

    #[test]
    fn path_attribute_points_mod_at_a_relative_file() {
        let got = imports("#[path = \"go.rs\"]\nmod go;\n");
        assert!(got.contains("relfile::go.rs"), "{got:?}");
        // The plain name-based edge must not also fire for a `#[path]` mod.
        assert!(!got.contains("mod::go"), "{got:?}");
    }

    #[test]
    fn include_macro_creates_a_relative_file_edge() {
        let got = imports("include!(\"sections/header.rs\");\n");
        assert!(got.contains("relfile::sections/header.rs"), "{got:?}");
    }

    #[test]
    fn include_str_is_not_a_module_edge() {
        let got = imports("const D: &str = include_str!(\"data.txt\");\n");
        assert!(got.iter().all(|i| !i.starts_with("relfile::")), "{got:?}");
    }

    #[test]
    fn plain_mod_declaration_still_resolves_by_name() {
        let got = imports("mod pattern;\n");
        assert!(got.contains("mod::pattern"), "{got:?}");
    }
}
