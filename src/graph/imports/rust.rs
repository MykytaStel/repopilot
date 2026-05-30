use std::collections::HashSet;
use tree_sitter::{Node, Tree};

pub(super) fn extract(tree: &Tree, content: &str) -> HashSet<String> {
    let mut result = HashSet::new();
    visit(tree.root_node(), content, &mut result);
    result
}

fn visit(node: Node<'_>, content: &str, result: &mut HashSet<String>) {
    match node.kind() {
        "use_declaration" => {
            if let Ok(text) = node.utf8_text(content.as_bytes()) {
                result.extend(rust_use_imports(text));
            }
        }
        "mod_item" => {
            if let Ok(text) = node.utf8_text(content.as_bytes())
                && let Some(module) = rust_mod_import(text)
            {
                result.insert(module);
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
