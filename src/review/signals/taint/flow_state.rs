use super::super::sources::SourceKind;
use std::collections::{HashMap, HashSet};
use tree_sitter::Node;

#[derive(Default)]
pub(super) struct TaintState {
    pub(super) names: HashMap<String, SourceKind>,
    paths: HashMap<String, SourceKind>,
    clean_paths: HashSet<String>,
}

impl TaintState {
    pub(super) fn mark_name(&mut self, name: &str, source: SourceKind) {
        self.names.insert(name.to_string(), source);
        self.remove_path_entries(name);
    }

    pub(super) fn clear_name(&mut self, name: &str) {
        self.names.remove(name);
        self.remove_path_entries(name);
    }

    pub(super) fn mark_path(&mut self, path: &str, source: SourceKind) {
        self.paths.insert(path.to_string(), source);
        self.clean_paths
            .retain(|clean| !path_is_same_or_descendant(clean, path));
    }

    pub(super) fn clear_path(&mut self, path: &str) {
        self.paths
            .retain(|tainted, _| !path_is_same_or_descendant(tainted, path));
        self.clean_paths.insert(path.to_string());
    }

    pub(super) fn is_path_explicitly_clean(&self, path: &str) -> bool {
        self.clean_paths
            .iter()
            .any(|clean| path_is_same_or_descendant(path, clean))
    }

    pub(super) fn source_for_path(&self, path: &str) -> Option<SourceKind> {
        if self.is_path_explicitly_clean(path) {
            return None;
        }

        if let Some(source) = self.paths.get(path) {
            return Some(*source);
        }
        if let Some((_, source)) = self
            .paths
            .iter()
            .filter(|(candidate, _)| path_is_same_or_descendant(path, candidate))
            .max_by_key(|(candidate, _)| candidate.len())
        {
            return Some(*source);
        }

        let root = path.split('.').next()?;
        self.names.get(root).copied()
    }

    fn remove_path_entries(&mut self, root: &str) {
        self.paths
            .retain(|path, _| !path_is_same_or_descendant(path, root));
        self.clean_paths
            .retain(|path| !path_is_same_or_descendant(path, root));
    }
}

pub(super) fn value_path(node: Node<'_>, content: &str) -> Option<String> {
    if node.kind() == "identifier" {
        return node
            .utf8_text(content.as_bytes())
            .ok()
            .map(ToOwned::to_owned);
    }
    access_path(node, content)
}

pub(super) fn access_path(node: Node<'_>, content: &str) -> Option<String> {
    if !matches!(
        node.kind(),
        "member_expression"
            | "member_access_expression"
            | "attribute"
            | "selector_expression"
            | "navigation_expression"
            | "subscript_expression"
    ) {
        return None;
    }
    normalize_access_path(node.utf8_text(content.as_bytes()).ok()?)
}

pub(super) fn normalize_access_path(text: &str) -> Option<String> {
    let normalized = text.trim().replace("?.[", "[").replace("?.", ".");
    if normalized.is_empty() || normalized.contains(['(', ')', ' ', '\t', '\n', '\r']) {
        return None;
    }

    let mut index = 0;
    let mut segments = vec![parse_identifier_segment(&normalized, &mut index)?];
    parse_access_suffixes(&normalized, &mut index, &mut segments)?;

    if segments.len() < 2 {
        return None;
    }
    Some(segments.join("."))
}

fn parse_access_suffixes(text: &str, index: &mut usize, segments: &mut Vec<String>) -> Option<()> {
    while *index < text.len() {
        match text.as_bytes()[*index] as char {
            '.' => {
                *index += 1;
                segments.push(parse_identifier_segment(text, index)?);
            }
            '[' => segments.push(parse_index_segment(text, index)?),
            _ => return None,
        }
    }
    Some(())
}

fn parse_identifier_segment(text: &str, index: &mut usize) -> Option<String> {
    let start = *index;
    while *index < text.len() && is_identifier_char(text.as_bytes()[*index] as char) {
        *index += 1;
    }
    (start != *index && is_identifier(&text[start..*index]))
        .then(|| text[start..*index].to_string())
}

fn parse_index_segment(text: &str, index: &mut usize) -> Option<String> {
    *index += 1;
    let start = *index;
    while *index < text.len() && (text.as_bytes()[*index] as char).is_ascii_digit() {
        *index += 1;
    }
    if start == *index || *index >= text.len() || text.as_bytes()[*index] as char != ']' {
        return None;
    }
    let segment = text[start..*index].to_string();
    *index += 1;
    Some(segment)
}

fn path_is_same_or_descendant(path: &str, parent: &str) -> bool {
    path == parent
        || path
            .strip_prefix(parent)
            .is_some_and(|suffix| suffix.starts_with('.'))
}

fn is_identifier_char(ch: char) -> bool {
    ch.is_ascii_alphanumeric() || matches!(ch, '_' | '$')
}

fn is_identifier(segment: &str) -> bool {
    let mut chars = segment.chars();
    let first = match chars.next() {
        Some(c) => c,
        None => return false,
    };
    (first.is_ascii_alphabetic() || matches!(first, '_' | '$')) && chars.all(is_identifier_char)
}
