//! Intra-procedural taint propagation.
//!
//! Two passes per function/module scope: seed tainted local names and exact
//! member paths from request-bound parameters and assignments, then report each
//! changed-line sink whose arguments carry taint. Whole-object taint remains the
//! conservative default, while exact member assignments can either add taint to
//! a clean object or clean one field of a tainted object.
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
use std::collections::{HashMap, HashSet};
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

#[derive(Default)]
struct TaintState {
    names: HashMap<String, SourceKind>,
    paths: HashMap<String, SourceKind>,
    clean_paths: HashSet<String>,
}

impl TaintState {
    fn mark_name(&mut self, name: &str, source: SourceKind) {
        self.names.insert(name.to_string(), source);
        self.remove_path_entries(name);
    }

    fn clear_name(&mut self, name: &str) {
        self.names.remove(name);
        self.remove_path_entries(name);
    }

    fn mark_path(&mut self, path: &str, source: SourceKind) {
        self.paths.insert(path.to_string(), source);
        self.clean_paths
            .retain(|clean| !path_is_same_or_descendant(clean, path));
    }

    fn clear_path(&mut self, path: &str) {
        self.paths
            .retain(|tainted, _| !path_is_same_or_descendant(tainted, path));
        self.clean_paths.insert(path.to_string());
    }

    fn source_for_path(&self, path: &str) -> Option<SourceKind> {
        if self
            .clean_paths
            .iter()
            .any(|clean| path_is_same_or_descendant(path, clean))
        {
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

fn path_is_same_or_descendant(path: &str, parent: &str) -> bool {
    path == parent
        || path
            .strip_prefix(parent)
            .is_some_and(|suffix| suffix.starts_with('.'))
}

// ── Seeding ─────────────────────────────────────────────────────────────────

fn seed_tainted(root: Node<'_>, content: &str, tables: &'static TaintTables) -> TaintState {
    let mut tainted = TaintState::default();
    collect_request_bound_parameters(root, content, tables, &mut tainted);

    let mut assignments: Vec<Assignment<'_>> = Vec::new();
    collect_assignments(root, tables, content, &mut assignments);
    assignments.sort_by_key(|assignment| assignment.rhs.start_byte());

    for assignment in &assignments {
        let source = node_has_source(assignment.rhs, content, tables)
            .or_else(|| node_mentions_tainted(assignment.rhs, content, tables, &tainted));

        for name in &assignment.names {
            match source {
                Some(kind) => tainted.mark_name(name, kind),
                None if !assignment.augmenting => tainted.clear_name(name),
                None => {}
            }
        }

        let rhs_path = value_path(assignment.rhs, content);
        for binding in &assignment.bindings {
            let binding_source = match rhs_path.as_deref() {
                Some(base) => tainted.source_for_path(&format!("{base}.{}", binding.path)),
                None => source,
            };
            match binding_source {
                Some(kind) => tainted.mark_name(&binding.name, kind),
                None if !assignment.augmenting => tainted.clear_name(&binding.name),
                None => {}
            }
        }

        if let Some(path) = &assignment.path {
            match source {
                Some(kind) => tainted.mark_path(path, kind),
                None if !assignment.augmenting => tainted.clear_path(path),
                None => {}
            }
        }
    }
    tainted
}

/// Seed request-controlled names that are bound directly by a framework
/// parameter decorator rather than by an assignment. A narrow set of NestJS
/// primitive parsing pipes is treated as an opaque clean boundary because its
/// output cannot carry string injection or path/command metacharacters.
fn collect_request_bound_parameters(
    node: Node<'_>,
    content: &str,
    tables: &'static TaintTables,
    out: &mut TaintState,
) {
    if is_parameter_node(node) {
        let text = node.utf8_text(content.as_bytes()).unwrap_or("");
        if is_nest_request_parameter(text)
            && !has_nest_primitive_pipe(text)
            && let Some(name) = parameter_binding_name(node, content)
        {
            out.mark_name(&name, SourceKind::HttpRequest);
        }
        return;
    }

    let mut cursor = node.walk();
    for child in node.named_children(&mut cursor) {
        if !(tables.is_flow_scope)(child) {
            collect_request_bound_parameters(child, content, tables, out);
        }
    }
}

fn is_parameter_node(node: Node<'_>) -> bool {
    matches!(
        node.kind(),
        "required_parameter" | "optional_parameter" | "rest_pattern"
    )
}

fn is_nest_request_parameter(text: &str) -> bool {
    ["Body", "Query", "Param", "Headers", "Req", "Request"]
        .iter()
        .any(|name| contains_decorator_token(text, name))
}

fn has_nest_primitive_pipe(text: &str) -> bool {
    [
        "ParseIntPipe",
        "ParseFloatPipe",
        "ParseBoolPipe",
        "ParseUUIDPipe",
    ]
    .iter()
    .any(|pipe| contains_identifier_token(text, pipe))
}

fn contains_decorator_token(text: &str, name: &str) -> bool {
    let marker = format!("@{name}");
    text.match_indices(&marker).any(|(start, _)| {
        let suffix = &text[start + marker.len()..];
        suffix
            .chars()
            .next()
            .is_none_or(|next| next == '(' || next.is_whitespace())
    })
}

fn contains_identifier_token(text: &str, identifier: &str) -> bool {
    text.match_indices(identifier).any(|(start, _)| {
        let before = text[..start].chars().next_back();
        let after = text[start + identifier.len()..].chars().next();
        before.is_none_or(|ch| !is_identifier_char(ch))
            && after.is_none_or(|ch| !is_identifier_char(ch))
    })
}

fn is_identifier_char(ch: char) -> bool {
    ch.is_ascii_alphanumeric() || matches!(ch, '_' | '$')
}

fn parameter_binding_name(node: Node<'_>, content: &str) -> Option<String> {
    for field in ["pattern", "name"] {
        if let Some(binding) = node.child_by_field_name(field)
            && let Some(name) = first_binding_identifier(binding, content)
        {
            return Some(name);
        }
    }

    let mut cursor = node.walk();
    node.named_children(&mut cursor)
        .filter(|child| !matches!(child.kind(), "decorator" | "type_annotation"))
        .find_map(|child| first_binding_identifier(child, content))
}

fn first_binding_identifier(node: Node<'_>, content: &str) -> Option<String> {
    if matches!(
        node.kind(),
        "identifier" | "shorthand_property_identifier_pattern"
    ) {
        return node
            .utf8_text(content.as_bytes())
            .ok()
            .map(ToOwned::to_owned);
    }
    if matches!(node.kind(), "decorator" | "type_annotation") {
        return None;
    }

    let mut cursor = node.walk();
    node.named_children(&mut cursor)
        .find_map(|child| first_binding_identifier(child, content))
}

struct DestructuredBinding {
    name: String,
    path: String,
}

struct Assignment<'a> {
    names: Vec<String>,
    bindings: Vec<DestructuredBinding>,
    path: Option<String>,
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
        let mut bindings = Vec::new();
        match lhs.kind() {
            "object_pattern" => collect_object_bindings(lhs, content, "", &mut bindings),
            "array_pattern" => collect_array_bindings(lhs, content, "", &mut bindings),
            _ => collect_lhs_names(lhs, content, &mut names),
        }
        let path = access_path(lhs, content);
        if !names.is_empty() || !bindings.is_empty() || path.is_some() {
            out.push(Assignment {
                names,
                bindings,
                path,
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

fn collect_object_bindings(
    node: Node<'_>,
    content: &str,
    prefix: &str,
    out: &mut Vec<DestructuredBinding>,
) {
    match node.kind() {
        "shorthand_property_identifier_pattern" => {
            if let Ok(name) = node.utf8_text(content.as_bytes()) {
                let path = join_binding_path(prefix, name);
                out.push(DestructuredBinding {
                    name: name.to_string(),
                    path,
                });
            }
        }
        "pair_pattern" => {
            let Some(key) = node.child_by_field_name("key") else {
                return;
            };
            let Some(value) = node.child_by_field_name("value") else {
                return;
            };
            let Ok(key_text) = key.utf8_text(content.as_bytes()) else {
                return;
            };
            if !is_identifier(key_text) {
                return;
            }
            let path = join_binding_path(prefix, key_text);
            collect_binding_value(value, content, &path, out);
        }
        "object_pattern" => {
            let mut cursor = node.walk();
            for child in node.named_children(&mut cursor) {
                collect_object_bindings(child, content, prefix, out);
            }
        }
        _ => {}
    }
}

fn collect_array_bindings(
    node: Node<'_>,
    content: &str,
    prefix: &str,
    out: &mut Vec<DestructuredBinding>,
) {
    let mut cursor = node.walk();
    for (index, child) in node.named_children(&mut cursor).enumerate() {
        if child.kind() == "rest_pattern" {
            continue;
        }
        let path = join_binding_path(prefix, &index.to_string());
        collect_binding_value(child, content, &path, out);
    }
}

fn collect_binding_value(
    node: Node<'_>,
    content: &str,
    path: &str,
    out: &mut Vec<DestructuredBinding>,
) {
    match node.kind() {
        "identifier" | "shorthand_property_identifier_pattern" => {
            if let Ok(name) = node.utf8_text(content.as_bytes()) {
                out.push(DestructuredBinding {
                    name: name.to_string(),
                    path: path.to_string(),
                });
            }
        }
        "object_pattern" => collect_object_bindings(node, content, path, out),
        "array_pattern" => collect_array_bindings(node, content, path, out),
        "assignment_pattern" => {
            if let Some(left) = node.child_by_field_name("left") {
                collect_binding_value(left, content, path, out);
            }
        }
        _ => {}
    }
}

fn join_binding_path(prefix: &str, segment: &str) -> String {
    if prefix.is_empty() {
        segment.to_string()
    } else {
        format!("{prefix}.{segment}")
    }
}

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

fn value_path(node: Node<'_>, content: &str) -> Option<String> {
    if node.kind() == "identifier" {
        return node
            .utf8_text(content.as_bytes())
            .ok()
            .map(ToOwned::to_owned);
    }
    access_path(node, content)
}

fn access_path(node: Node<'_>, content: &str) -> Option<String> {
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

fn normalize_access_path(text: &str) -> Option<String> {
    let normalized = text.trim().replace("?.", ".");
    if normalized.is_empty()
        || normalized.contains(['(', ')', ' ', '\t', '\n', '\r'])
    {
        return None;
    }

    let bytes = normalized.as_bytes();
    let mut index = 0;
    let mut segments = Vec::new();

    let first_start = index;
    while index < bytes.len() && is_identifier_char(bytes[index] as char) {
        index += 1;
    }
    if first_start == index || !is_identifier(&normalized[first_start..index]) {
        return None;
    }
    segments.push(normalized[first_start..index].to_string());

    while index < bytes.len() {
        match bytes[index] as char {
            '.' => {
                index += 1;
                let start = index;
                while index < bytes.len() && is_identifier_char(bytes[index] as char) {
                    index += 1;
                }
                if start == index || !is_identifier(&normalized[start..index]) {
                    return None;
                }
                segments.push(normalized[start..index].to_string());
            }
            '[' => {
                index += 1;
                let start = index;
                while index < bytes.len() && (bytes[index] as char).is_ascii_digit() {
                    index += 1;
                }
                if start == index || index >= bytes.len() || bytes[index] as char != ']' {
                    return None;
                }
                segments.push(normalized[start..index].to_string());
                index += 1;
            }
            _ => return None,
        }
    }

    if segments.len() < 2 {
        return None;
    }
    Some(segments.join("."))
}

fn is_identifier(segment: &str) -> bool {
    let mut chars = segment.chars();
    let first = match chars.next() {
        Some(c) => c,
        None => return false,
    };
    (first.is_ascii_alphabetic() || matches!(first, '_' | '$')) && chars.all(is_identifier_char)
}

fn node_mentions_tainted(
    node: Node<'_>,
    content: &str,
    tables: &'static TaintTables,
    tainted: &TaintState,
) -> Option<SourceKind> {
    if (tables.is_flow_scope)(node) {
        return None;
    }
    if super::sanitizers::is_sanitizer_call(node, content, tables) {
        return None;
    }
    if let Some(path) = access_path(node, content) {
        return tainted.source_for_path(&path);
    }
    if node.kind() == "identifier" {
        let text = node.utf8_text(content.as_bytes()).ok()?;
        if let Some(source) = tainted.names.get(text) {
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
    tainted: &TaintState,
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
    tainted: &TaintState,
) -> Option<SourceKind> {
    match sink.kind {
        SinkKind::Sql => sql_taint(sink.args, content, tables, tainted),
        _ => node_has_source(sink.args, content, tables)
            .or_else(|| node_mentions_tainted(sink.args, content, tables, tainted)),
    }
}

fn sql_taint(
    args: Node<'_>,
    content: &str,
    tables: &'static TaintTables,
    tainted: &TaintState,
) -> Option<SourceKind> {
    let first = first_named_arg(args)?;
    let first_text = first.utf8_text(content.as_bytes()).unwrap_or("");

    if (tables.is_string_building)(first, content) {
        return node_has_source(first, content, tables)
            .or_else(|| node_mentions_tainted(first, content, tables, tainted));
    }
    if first.kind() == "identifier" {
        return tainted.names.get(first_text).copied();
    }
    node_has_source(first, content, tables)
        .or_else(|| node_mentions_tainted(first, content, tables, tainted))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn primitive_nest_pipes_are_recognized_with_token_boundaries() {
        for pipe in [
            "ParseIntPipe",
            "ParseFloatPipe",
            "ParseBoolPipe",
            "ParseUUIDPipe",
        ] {
            let parameter = format!("@Param(\"id\", {pipe}) id: string");
            assert!(has_nest_primitive_pipe(&parameter), "missing {pipe}");
        }

        assert!(!has_nest_primitive_pipe(
            "@Param(\"id\", MyParseIntPipe) id: string"
        ));
    }

    #[test]
    fn configurable_and_custom_nest_pipes_remain_tainted() {
        for pipe in ["ValidationPipe", "ParseEnumPipe", "CustomPipe"] {
            let parameter = format!("@Body({pipe}) body: Payload");
            assert!(!has_nest_primitive_pipe(&parameter), "unexpected {pipe}");
        }
    }

    #[test]
    fn exact_tainted_path_on_clean_object_is_tracked() {
        let mut state = TaintState::default();
        state.mark_path("target.filename", SourceKind::HttpRequest);

        assert_eq!(
            state.source_for_path("target.filename"),
            Some(SourceKind::HttpRequest)
        );
        assert_eq!(state.source_for_path("target.label"), None);
    }

    #[test]
    fn clean_path_overrides_whole_object_taint() {
        let mut state = TaintState::default();
        state.mark_name("body", SourceKind::HttpRequest);
        state.clear_path("body.filename");

        assert_eq!(state.source_for_path("body.filename"), None);
        assert_eq!(
            state.source_for_path("body.command"),
            Some(SourceKind::HttpRequest)
        );
    }

    #[test]
    fn clearing_parent_path_cleans_nested_members() {
        let mut state = TaintState::default();
        state.mark_name("body", SourceKind::HttpRequest);
        state.clear_path("body.user");

        assert_eq!(state.source_for_path("body.user.id"), None);
        assert_eq!(
            state.source_for_path("body.account.id"),
            Some(SourceKind::HttpRequest)
        );
    }

    #[test]
    fn access_paths_accept_static_indexes_and_reject_dynamic_indexes() {
        assert_eq!(
            normalize_access_path("body.user.id"),
            Some("body.user.id".into())
        );
        assert_eq!(
            normalize_access_path("body?.items?.[0].command"),
            Some("body.items.0.command".into())
        );
        assert_eq!(
            normalize_access_path("items[12]"),
            Some("items.12".into())
        );
        assert_eq!(normalize_access_path("body[userKey]"), None);
        assert_eq!(normalize_access_path("items[-1]"), None);
        assert_eq!(normalize_access_path("getBody().id"), None);
    }

    #[test]
    fn clean_destructured_property_does_not_inherit_whole_object_taint() {
        let mut state = TaintState::default();
        state.mark_name("body", SourceKind::HttpRequest);
        state.clear_path("body.id");

        assert_eq!(state.source_for_path("body.id"), None);
        assert_eq!(
            state.source_for_path("body.command"),
            Some(SourceKind::HttpRequest)
        );
    }

    #[test]
    fn clean_static_index_does_not_clean_siblings() {
        let mut state = TaintState::default();
        state.mark_name("items", SourceKind::HttpRequest);
        state.clear_path("items.0");

        assert_eq!(state.source_for_path("items.0"), None);
        assert_eq!(
            state.source_for_path("items.1"),
            Some(SourceKind::HttpRequest)
        );
    }

    #[test]
    fn exact_tainted_index_on_clean_array_is_tracked() {
        let mut state = TaintState::default();
        state.mark_path("items.0", SourceKind::HttpRequest);

        assert_eq!(
            state.source_for_path("items.0"),
            Some(SourceKind::HttpRequest)
        );
        assert_eq!(state.source_for_path("items.1"), None);
    }
}
