use super::super::{sources::SourceKind, tables::TaintTables};
use super::TaintSignal;
use crate::review::diff::ChangedFile;
use std::collections::BTreeMap;
use tree_sitter::Node;

pub(super) type LocalHelperReturns = BTreeMap<(usize, usize), Option<SourceKind>>;

struct ResolvedCall<'tree, 'helpers> {
    node: Node<'tree>,
    helper: &'helpers Helper<'tree>,
    arguments: Vec<Node<'tree>>,
}

pub(super) struct Helper<'a> {
    pub(super) name: String,
    pub(super) node: Node<'a>,
    pub(super) parameters: Vec<String>,
    pub(super) return_flow: ReturnFlow,
}

pub(super) struct Context<'a> {
    pub(super) content: &'a str,
    pub(super) tables: &'static TaintTables,
    pub(super) file: &'a ChangedFile,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum ReturnFlow {
    Parameter(usize),
    Clean,
    Unknown,
}

const MAX_SCOPES: usize = 512;
const MAX_CALLS_PER_SCOPE: usize = 512;
pub(super) const MAX_HELPERS: usize = 128;

#[path = "helper_summary/calls.rs"]
mod calls;
#[path = "helper_summary/declarations.rs"]
mod declarations;

pub(super) fn detect(
    root: Node<'_>,
    content: &str,
    tables: &'static TaintTables,
    file: &ChangedFile,
    out: &mut Vec<TaintSignal>,
) -> BTreeMap<usize, LocalHelperReturns> {
    let context = Context {
        content,
        tables,
        file,
    };
    let valid = valid_helpers(root, content, tables);
    if valid.is_empty() {
        return BTreeMap::new();
    }
    let mut callers = vec![root];
    calls::collect_scopes(root, tables, &mut callers);
    if callers.len() > MAX_SCOPES {
        return BTreeMap::new();
    }
    let mut summarized = BTreeMap::new();
    for caller in callers {
        let names = signals_for_caller(caller, &valid, &context, out);
        if !names.is_empty() {
            summarized.insert(caller.id(), names);
        }
    }
    summarized
}

fn valid_helpers<'a>(
    root: Node<'a>,
    content: &str,
    tables: &'static TaintTables,
) -> Vec<Helper<'a>> {
    let helpers = declarations::unique_helpers(root, content, tables);
    let names = helpers.iter().map(|helper| helper.name.clone()).collect();
    helpers
        .into_iter()
        .filter(|helper| {
            !declarations::unsupported_control_flow(helper.node, tables)
                && !declarations::calls_local_helper(helper.node, content, tables, &names)
        })
        .collect()
}

fn signals_for_caller(
    caller: Node<'_>,
    helpers: &[Helper<'_>],
    context: &Context<'_>,
    out: &mut Vec<TaintSignal>,
) -> LocalHelperReturns {
    if helpers.iter().any(|helper| helper.node.id() == caller.id()) {
        return BTreeMap::new();
    }
    let mut calls = Vec::new();
    calls::collect_direct_calls(caller, caller, context.content, context.tables, &mut calls);
    if calls.len() > MAX_CALLS_PER_SCOPE {
        return BTreeMap::new();
    }
    let resolved = calls
        .into_iter()
        .filter_map(|node| resolve_call(node, caller, helpers, context))
        .collect::<Vec<_>>();
    // A second summarized call cannot inherit taint from another recognized
    // helper's result. The ordinary caller pass uses the evaluated results.
    let blocked_returns = resolved
        .iter()
        .map(|call| ((call.node.start_byte(), call.node.end_byte()), None))
        .collect();
    let mut returns = BTreeMap::new();
    for call in resolved {
        let state = super::flow_seed::seed_tainted_before(
            caller,
            context.content,
            context.tables,
            &blocked_returns,
            call.node.start_byte(),
        );
        let result =
            calls::apply_helper_call(&call, caller, &state, &blocked_returns, context, out);
        returns.insert((call.node.start_byte(), call.node.end_byte()), result);
    }
    returns
}

fn resolve_call<'tree, 'helpers>(
    node: Node<'tree>,
    caller: Node<'tree>,
    helpers: &'helpers [Helper<'tree>],
    context: &Context<'_>,
) -> Option<ResolvedCall<'tree, 'helpers>> {
    let name = calls::direct_callee(node, context.content)?;
    let helper = helpers.iter().find(|helper| helper.name == name)?;
    if calls::caller_shadowed(caller, helper, context.content, context.tables) {
        return None;
    }
    let arguments = calls::bound_arguments(node, &helper.parameters, context.content)?;
    Some(ResolvedCall {
        node,
        helper,
        arguments,
    })
}
