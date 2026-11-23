use super::super::super::sources::SourceKind;
use super::super::flow_state::{TaintState, value_path};
use super::{Assignment, DestructuredBinding};

pub(super) fn apply_name_assignments(
    assignment: &Assignment<'_>,
    source: Option<SourceKind>,
    tainted: &mut TaintState,
) {
    for name in &assignment.names {
        apply_name(assignment, name, source, tainted);
    }
}

pub(super) fn apply_binding_assignments(
    assignment: &Assignment<'_>,
    content: &str,
    source: Option<SourceKind>,
    tainted: &mut TaintState,
) {
    let rhs_path = value_path(assignment.rhs, content);
    for binding in &assignment.bindings {
        let binding_source = source_for_binding(binding, rhs_path.as_deref(), source, tainted);
        apply_name(assignment, &binding.name, binding_source, tainted);
    }
}

fn source_for_binding(
    binding: &DestructuredBinding,
    rhs_path: Option<&str>,
    source: Option<SourceKind>,
    tainted: &TaintState,
) -> Option<SourceKind> {
    let Some(base) = rhs_path else {
        return source;
    };
    let path = format!("{base}.{}", binding.path);
    if tainted.is_path_explicitly_clean(&path) {
        return None;
    }
    tainted.source_for_path(&path).or(source)
}

fn apply_name(
    assignment: &Assignment<'_>,
    name: &str,
    source: Option<SourceKind>,
    tainted: &mut TaintState,
) {
    match source {
        Some(kind) => tainted.mark_name(name, kind),
        None if !assignment.augmenting => tainted.clear_name(name),
        None => {}
    }
}

pub(super) fn apply_path_assignment(
    assignment: &Assignment<'_>,
    source: Option<SourceKind>,
    tainted: &mut TaintState,
) {
    if let Some(path) = &assignment.path {
        match source {
            Some(kind) => tainted.mark_path(path, kind),
            None if !assignment.augmenting => tainted.clear_path(path),
            None => {}
        }
    }
}
