use super::super::ast::first_named_arg;
use super::super::sinks::{Sink, SinkKind};
use super::super::sources::node_has_source;
use super::super::tables::TaintTables;
use super::TaintSignal;
use super::flow_assignments::{
    Assignment, apply_assignment_with_local_helpers, collect_assignments,
};
use super::flow_seed::seed_tainted_parameters;
use super::flow_state::{TaintState, access_path};
use super::helper_summary::LocalHelperReturns;
use crate::review::diff::ChangedFile;
use crate::review::signals::behavioral::truncate_str;
use tree_sitter::Node;

pub(super) struct SinkContext<'a> {
    pub(super) content: &'a str,
    pub(super) tables: &'static TaintTables,
    pub(super) file: &'a ChangedFile,
    pub(super) local_helpers: &'a LocalHelperReturns,
}

pub(super) fn check_sinks_in_order(
    scope: Node<'_>,
    content: &str,
    tables: &'static TaintTables,
    file: &ChangedFile,
    local_helpers: &LocalHelperReturns,
    out: &mut Vec<TaintSignal>,
) {
    let mut tainted = seed_tainted_parameters(scope, content, tables);
    let context = SinkContext {
        content,
        tables,
        file,
        local_helpers,
    };
    for (_, _, event) in flow_events(scope, content, tables) {
        match event {
            FlowEvent::Assignment(assignment) => {
                apply_assignment(&assignment, &context, &mut tainted)
            }
            FlowEvent::Sink(node) => check_one_sink(node, &context, &tainted, None, out),
        }
    }
}

pub(super) fn check_sinks_triggered_by_call(
    scope: Node<'_>,
    context: &SinkContext<'_>,
    [mut baseline, mut propagated]: [TaintState; 2],
    after_byte: Option<usize>,
    call_line: usize,
    out: &mut Vec<TaintSignal>,
) {
    for (position, _, event) in flow_events(scope, context.content, context.tables) {
        if after_byte.is_some_and(|after| position <= after) {
            continue;
        }
        match event {
            FlowEvent::Assignment(assignment) => {
                apply_assignment(&assignment, context, &mut baseline);
                apply_assignment(&assignment, context, &mut propagated);
            }
            FlowEvent::Sink(node) if assessed_sink(node, context, &baseline).is_none() => {
                check_one_sink(node, context, &propagated, Some(call_line), out);
            }
            FlowEvent::Sink(_) => {}
        }
    }
}

fn apply_assignment(
    assignment: &Assignment<'_>,
    context: &SinkContext<'_>,
    state: &mut TaintState,
) {
    apply_assignment_with_local_helpers(
        assignment,
        context.content,
        context.tables,
        state,
        context.local_helpers,
    );
}

fn flow_events<'a>(
    scope: Node<'a>,
    content: &str,
    tables: &'static TaintTables,
) -> Vec<(usize, u8, FlowEvent<'a>)> {
    let mut events = Vec::new();
    let mut assignments = Vec::new();
    collect_assignments(scope, tables, content, &mut assignments);
    events.extend(assignments.into_iter().map(|assignment| {
        (
            assignment.rhs.end_byte(),
            1_u8,
            FlowEvent::Assignment(assignment),
        )
    }));
    collect_sink_events(scope, scope, content, tables, &mut events);
    events.sort_by_key(|(position, order, _)| (*position, *order));
    events
}

enum FlowEvent<'a> {
    Assignment(Assignment<'a>),
    Sink(Node<'a>),
}

fn collect_sink_events<'a>(
    node: Node<'a>,
    scope: Node<'a>,
    content: &str,
    tables: &'static TaintTables,
    out: &mut Vec<(usize, u8, FlowEvent<'a>)>,
) {
    if node.id() != scope.id() && (tables.is_flow_scope)(node) {
        return;
    }
    if (tables.classify_sink)(node, content).is_some() {
        out.push((node.start_byte(), 0, FlowEvent::Sink(node)));
    }
    let mut cursor = node.walk();
    for child in node.children(&mut cursor) {
        collect_sink_events(child, scope, content, tables, out);
    }
}

fn check_one_sink(
    node: Node<'_>,
    context: &SinkContext<'_>,
    tainted: &TaintState,
    trigger_line: Option<usize>,
    out: &mut Vec<TaintSignal>,
) {
    let line = node.start_position().row + 1;
    if !(context.file.contains_line(line)
        || trigger_line.is_some_and(|line| context.file.contains_line(line)))
    {
        return;
    }
    if let Some((sink, source)) = assessed_sink(node, context, tainted) {
        let call_text = node.utf8_text(context.content.as_bytes()).unwrap_or("");
        out.push(TaintSignal {
            source,
            sink,
            path: context.file.path_string(),
            line,
            detail: format!(
                "{} reaches {}: {}",
                source.label(),
                sink.label(),
                truncate_str(call_text, 60)
            ),
        });
    }
}

fn assessed_sink(
    node: Node<'_>,
    context: &SinkContext<'_>,
    tainted: &TaintState,
) -> Option<(SinkKind, super::super::sources::SourceKind)> {
    let sink = (context.tables.classify_sink)(node, context.content)?;
    let source = sink_taint(&sink, context, tainted)?;
    Some((sink.kind, source))
}

fn sink_taint(
    sink: &Sink<'_>,
    context: &SinkContext<'_>,
    tainted: &TaintState,
) -> Option<super::super::sources::SourceKind> {
    let node = match sink.kind {
        SinkKind::Sql => first_named_arg(sink.args)?,
        SinkKind::Exec => sink.safe_exec_command.unwrap_or(sink.args),
        _ => sink.args,
    };
    if sink.kind == SinkKind::Sql
        && node.kind() == "identifier"
        && !(context.tables.is_string_building)(node, context.content)
    {
        let name = node.utf8_text(context.content.as_bytes()).ok()?;
        return tainted.names.get(name).copied();
    }
    expression_source(
        node,
        context.content,
        context.tables,
        tainted,
        context.local_helpers,
    )
}

pub(super) fn expression_source(
    node: Node<'_>,
    content: &str,
    tables: &'static TaintTables,
    tainted: &TaintState,
    returns: &LocalHelperReturns,
) -> Option<super::super::sources::SourceKind> {
    if let Some(source) = returns.get(&(node.start_byte(), node.end_byte())) {
        return *source;
    }
    if returns.is_empty()
        || returns
            .range((node.start_byte(), 0)..(node.end_byte(), 0))
            .next()
            .is_none()
    {
        return node_has_source(node, content, tables)
            .or_else(|| node_mentions_tainted(node, content, tables, tainted));
    }
    if (tables.is_flow_scope)(node)
        || super::super::sanitizers::is_sanitizer_call(node, content, tables)
    {
        return None;
    }
    let mut cursor = node.walk();
    node.named_children(&mut cursor)
        .find_map(|child| expression_source(child, content, tables, tainted, returns))
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
