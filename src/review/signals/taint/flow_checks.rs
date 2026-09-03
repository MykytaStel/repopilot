use super::super::ast::first_named_arg;
use super::super::sinks::{Sink, SinkKind};
use super::super::sources::node_has_source;
use super::super::tables::TaintTables;
use super::TaintSignal;
use super::flow_state::{TaintState, access_path};
use crate::review::diff::ChangedFile;
use crate::review::signals::behavioral::truncate_str;
use tree_sitter::Node;

pub(super) fn check_sinks(
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
) -> Option<super::super::sources::SourceKind> {
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
) -> Option<super::super::sources::SourceKind> {
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

pub(super) fn node_mentions_tainted(
    node: Node<'_>,
    content: &str,
    tables: &'static TaintTables,
    tainted: &TaintState,
) -> Option<super::super::sources::SourceKind> {
    if (tables.is_flow_scope)(node) {
        return None;
    }
    if super::super::sanitizers::is_sanitizer_call(node, content, tables) {
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
