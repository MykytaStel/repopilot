use super::super::super::TaintSignal;
use super::super::super::tables::TaintTables;
use super::super::flow_state::TaintState;
use super::{Origin, ambiguous_statement, executions, update};
use crate::review::diff::ChangedFile;
use std::collections::BTreeMap;
use tree_sitter::Node;

type BranchState = (
    TaintState,
    BTreeMap<String, Origin>,
    BTreeMap<String, String>,
);

pub(super) struct BranchContext<'a> {
    pub(super) content: &'a str,
    pub(super) tables: &'static TaintTables,
    pub(super) file: &'a ChangedFile,
    pub(super) out: &'a mut Vec<TaintSignal>,
}

pub(super) fn merge_if_statement(
    node: Node<'_>,
    context: &mut BranchContext<'_>,
    state: &mut TaintState,
    origins: &mut BTreeMap<String, Origin>,
    aliases: &mut BTreeMap<String, String>,
) -> bool {
    let Some(condition) = node.child_by_field_name("condition") else {
        return false;
    };
    let (mut assignments, mut execution, mut conditional) = (0, false, false);
    super::expression_facts(
        condition,
        context.content,
        context.tables,
        &mut assignments,
        &mut execution,
        &mut conditional,
    );
    if assignments > 0 || execution || conditional {
        return false;
    }
    let Some(consequence) = node.child_by_field_name("consequence") else {
        return false;
    };
    let unchanged = (state.clone(), origins.clone(), aliases.clone());
    let mut branches = vec![unchanged.clone()];
    if !process_branch(consequence, context, &mut branches[0]) {
        return false;
    }
    if let Some(alternative) = node.child_by_field_name("alternative") {
        let mut branch = (state.clone(), origins.clone(), aliases.clone());
        if !process_branch(alternative, context, &mut branch) {
            return false;
        }
        branches.push(branch);
    } else {
        branches.push(unchanged);
    }
    if branches
        .iter()
        .skip(1)
        .any(|branch| branch.2 != branches[0].2)
    {
        return false;
    }
    *state = TaintState::merge_possible(
        &branches
            .iter()
            .map(|branch| branch.0.clone())
            .collect::<Vec<_>>(),
    );
    *origins = merge_origins(&branches);
    *aliases = branches[0].2.clone();
    true
}

fn process_branch(
    branch: Node<'_>,
    context: &mut BranchContext<'_>,
    state: &mut BranchState,
) -> bool {
    let statements = if branch.kind() == "block" {
        let mut cursor = branch.walk();
        branch.named_children(&mut cursor).collect::<Vec<_>>()
    } else {
        vec![branch]
    };
    for statement in statements {
        if !matches!(
            statement.kind(),
            "local_declaration_statement" | "expression_statement"
        ) || ambiguous_statement(statement, context.content, context.tables)
        {
            return false;
        }
        update(
            statement,
            context.content,
            context.tables,
            &mut state.0,
            &mut state.1,
            &mut state.2,
        );
        executions(
            statement,
            context.content,
            context.tables,
            context.file,
            &state.1,
            &state.2,
            context.out,
        );
    }
    true
}

fn merge_origins(branches: &[BranchState]) -> BTreeMap<String, Origin> {
    let mut merged = BTreeMap::new();
    for (_, origins, _) in branches {
        for (path, origin) in origins {
            merged
                .entry(path.clone())
                .and_modify(|current: &mut Origin| *current = (*current).min(*origin))
                .or_insert(*origin);
        }
    }
    merged
}
