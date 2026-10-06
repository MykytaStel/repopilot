//! Conservative straight-line CommandText state; no heap aliases or CFG joins.
use super::super::{SinkKind, SourceKind, TaintSignal};
use super::super::{sources::node_has_source, tables::TaintTables};
use super::flow_assignments::{apply_assignment, collect_assignments};
use super::flow_checks::node_mentions_tainted;
use super::flow_state::TaintState;
use crate::review::diff::ChangedFile;
use std::collections::BTreeMap;
use tree_sitter::Node;

type Origin = (SourceKind, usize, usize);

pub(super) fn detect(
    root: Node<'_>,
    content: &str,
    tables: &'static TaintTables,
    file: &ChangedFile,
    out: &mut Vec<TaintSignal>,
) {
    if !file.path_string().ends_with(".cs") || !(tables.is_flow_scope)(root) {
        return;
    }
    let Some(body) = root
        .child_by_field_name("body")
        .filter(|n| n.kind() == "block")
    else {
        return;
    };
    let mut state = TaintState::default();
    let mut origins = BTreeMap::new();
    let mut cursor = body.walk();
    for statement in body.named_children(&mut cursor) {
        if !matches!(
            statement.kind(),
            "local_declaration_statement" | "expression_statement"
        ) {
            state = TaintState::default();
            origins.clear();
            continue;
        }
        update(statement, content, tables, &mut state, &mut origins);
        executions(statement, content, tables, file, &origins, out);
    }
}

fn update(
    node: Node<'_>,
    content: &str,
    tables: &'static TaintTables,
    state: &mut TaintState,
    origins: &mut BTreeMap<String, Origin>,
) {
    let mut assignments = Vec::new();
    collect_assignments(node, tables, content, &mut assignments);
    for assignment in assignments {
        let line = assignment.rhs.start_position().row + 1;
        let direct = node_has_source(assignment.rhs, content, tables);
        let source =
            direct.or_else(|| node_mentions_tainted(assignment.rhs, content, tables, state));
        let source_line = if direct.is_some() {
            line
        } else {
            origin_line(assignment.rhs, content, tables, origins).unwrap_or(line)
        };
        for name in &assignment.names {
            origins.remove(&format!("{name}.CommandText"));
            if let Some(kind) = source {
                origins.insert(name.clone(), (kind, source_line, line));
            } else if !assignment.augmenting {
                origins.remove(name);
            }
        }
        if let Some(path) = &assignment.path
            && path.ends_with(".CommandText")
            && path.split('.').count() == 2
        {
            if let Some(kind) = source {
                origins.insert(path.clone(), (kind, source_line, line));
            } else if !assignment.augmenting {
                origins.remove(path);
            }
        }
        apply_assignment(&assignment, content, tables, state);
    }
}

fn executions(
    node: Node<'_>,
    content: &str,
    tables: &'static TaintTables,
    file: &ChangedFile,
    origins: &BTreeMap<String, Origin>,
    out: &mut Vec<TaintSignal>,
) {
    if (tables.is_flow_scope)(node) {
        return;
    }
    if node.kind() == "invocation_expression"
        && let Some(sink) = (tables.classify_sink)(node, content)
        && sink.kind == SinkKind::Sql
    {
        let callee = content
            .get(node.start_byte()..sink.args.start_byte())
            .unwrap_or("")
            .trim();
        if let Some((receiver, _)) = callee.rsplit_once('.')
            && !receiver.contains(['.', '(', ')', '[', ']'])
            && let Some((source, source_line, assignment_line)) =
                origins.get(&format!("{receiver}.CommandText"))
        {
            let line = node.start_position().row + 1;
            if file.contains_line(line)
                || file.contains_line(*assignment_line)
                || file.contains_line(*source_line)
            {
                out.push(TaintSignal { source: *source, sink: SinkKind::Sql,
                    path: file.path_string(), line,
                    detail: format!("{} at line {} reaches {}.CommandText at line {} then {} at line {} (straight-line local receiver)",
                        source.label(), source_line, receiver, assignment_line, callee, line) });
            }
        }
    }
    let mut cursor = node.walk();
    for child in node.named_children(&mut cursor) {
        executions(child, content, tables, file, origins, out);
    }
}

fn origin_line(
    node: Node<'_>,
    content: &str,
    tables: &'static TaintTables,
    origins: &BTreeMap<String, Origin>,
) -> Option<usize> {
    if (tables.is_flow_scope)(node) {
        return None;
    }
    if node.kind() == "identifier" {
        return origins
            .get(node.utf8_text(content.as_bytes()).ok()?)
            .map(|origin| origin.1);
    }
    let mut cursor = node.walk();
    node.named_children(&mut cursor)
        .find_map(|child| origin_line(child, content, tables, origins))
}
