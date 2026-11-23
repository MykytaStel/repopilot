use super::super::super::super::{sources::SourceKind, tables::TaintTables};
use super::super::super::{
    flow_checks::{SinkContext, check_sinks_triggered_by_call, expression_source},
    flow_seed::seed_tainted_parameters,
    flow_state::TaintState,
};
use super::super::{Context, LocalHelperReturns, ResolvedCall, ReturnFlow};
use crate::review::signals::taint::TaintSignal;
use tree_sitter::Node;

pub(in crate::review::signals::taint::flow::helper_summary) fn apply_helper_call(
    call: &ResolvedCall<'_, '_>,
    caller: Node<'_>,
    caller_state: &TaintState,
    blocked_returns: &LocalHelperReturns,
    context: &Context<'_>,
    out: &mut Vec<TaintSignal>,
) -> Option<SourceKind> {
    let input = helper_input(call, caller_state, blocked_returns, context);
    let returned = match call.helper.return_flow {
        ReturnFlow::Parameter(index) => input.incoming[index],
        ReturnFlow::Clean => None,
        ReturnFlow::Unknown => input.incoming.iter().flatten().copied().next(),
    };
    if !input.inherited.is_empty() {
        emit_helper_sinks(call, &input, context, out);
    }
    if let (ReturnFlow::Parameter(index), Some(source)) = (call.helper.return_flow, returned) {
        let sink_context = SinkContext {
            content: context.content,
            tables: context.tables,
            file: context.file,
            local_helpers: blocked_returns,
        };
        emit_return_sinks(
            call,
            &input,
            caller,
            caller_state,
            (index, source),
            &sink_context,
            out,
        );
    }
    returned
}

struct HelperInput {
    inherited: Vec<(String, SourceKind)>,
    evidence: String,
    incoming: Vec<Option<SourceKind>>,
}

fn helper_input(
    call: &ResolvedCall<'_, '_>,
    caller_state: &TaintState,
    blocked_returns: &LocalHelperReturns,
    context: &Context<'_>,
) -> HelperInput {
    let incoming = call
        .arguments
        .iter()
        .map(|argument| {
            expression_source(
                *argument,
                context.content,
                context.tables,
                caller_state,
                blocked_returns,
            )
        })
        .collect::<Vec<_>>();
    let mut inherited = Vec::new();
    let mut evidence = Vec::new();
    for ((parameter, source), argument) in call
        .helper
        .parameters
        .iter()
        .zip(&incoming)
        .zip(&call.arguments)
    {
        if let Some(source) = source {
            inherited.push((parameter.clone(), *source));
            evidence.push(format!(
                "parameter `{parameter}` from line {}",
                argument.start_position().row + 1
            ));
        }
    }
    HelperInput {
        inherited,
        evidence: evidence.join(", "),
        incoming,
    }
}

fn emit_helper_sinks(
    call: &ResolvedCall<'_, '_>,
    input: &HelperInput,
    context: &Context<'_>,
    out: &mut Vec<TaintSignal>,
) {
    let helper = call.helper;
    let call_line = call.node.start_position().row + 1;
    let local_helpers = LocalHelperReturns::new();
    let sink_context = SinkContext {
        content: context.content,
        tables: context.tables,
        file: context.file,
        local_helpers: &local_helpers,
    };
    let baseline = seed_tainted_parameters(helper.node, context.content, context.tables);
    let mut inherited = baseline.clone();
    for (name, source) in &input.inherited {
        inherited.mark_name(name, *source);
    }
    let first = out.len();
    check_sinks_triggered_by_call(
        helper.node,
        &sink_context,
        [baseline, inherited],
        None,
        call_line,
        out,
    );
    for signal in &mut out[first..] {
        signal.detail.push_str(&format!(
            "; local helper `{}` receives tainted input in {} at line {call_line}",
            helper.name, input.evidence
        ));
    }
}

fn emit_return_sinks(
    call: &ResolvedCall<'_, '_>,
    input: &HelperInput,
    caller: Node<'_>,
    caller_state: &TaintState,
    returned: (usize, SourceKind),
    context: &SinkContext<'_>,
    out: &mut Vec<TaintSignal>,
) {
    let Some(target) = assigned_call_target(call.node, caller, context.content, context.tables)
    else {
        return;
    };
    let mut baseline = caller_state.clone();
    baseline.clear_name(&target);
    let mut inherited = baseline.clone();
    inherited.mark_name(&target, returned.1);
    let call_line = call.node.start_position().row + 1;
    let first = out.len();
    check_sinks_triggered_by_call(
        caller,
        context,
        [baseline, inherited],
        Some(call.node.end_byte()),
        call_line,
        out,
    );
    for signal in &mut out[first..] {
        signal.detail.push_str(&format!(
            "; local helper `{}` returns tainted parameter `{}` into `{target}` from {} at line {call_line}",
            call.helper.name, call.helper.parameters[returned.0], input.evidence
        ));
    }
}

fn assigned_call_target(
    call: Node<'_>,
    scope: Node<'_>,
    content: &str,
    tables: &'static TaintTables,
) -> Option<String> {
    let mut current = call.parent();
    while let Some(node) = current {
        if node.id() == scope.id() {
            return None;
        }
        current = node.parent();
        if !tables.assignment_kinds.contains(&node.kind()) {
            continue;
        }
        if let Some(target) = direct_assignment_target(node, call, content) {
            return Some(target);
        }
    }
    None
}

fn direct_assignment_target(assignment: Node<'_>, call: Node<'_>, content: &str) -> Option<String> {
    let (lhs, rhs) = assignment_parts(assignment)?;
    if !is_direct_rhs(rhs, call) || lhs.kind() != "identifier" {
        return None;
    }
    Some(lhs.utf8_text(content.as_bytes()).ok()?.to_string())
}

fn assignment_parts(assignment: Node<'_>) -> Option<(Node<'_>, Node<'_>)> {
    if assignment.kind() == "variable_declarator" {
        Some((
            assignment.child_by_field_name("name")?,
            assignment.child_by_field_name("value")?,
        ))
    } else {
        Some((
            assignment.child_by_field_name("left")?,
            assignment.child_by_field_name("right")?,
        ))
    }
}

fn is_direct_rhs(rhs: Node<'_>, call: Node<'_>) -> bool {
    rhs.id() == call.id()
        || (rhs.kind() == "expression_list"
            && rhs.named_child_count() == 1
            && rhs
                .named_child(0)
                .is_some_and(|child| child.id() == call.id()))
}
