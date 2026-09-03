//! Intra-procedural taint propagation.

#[path = "flow_assignments.rs"]
mod flow_assignments;
#[path = "flow_checks.rs"]
mod flow_checks;
#[path = "flow_seed.rs"]
mod flow_seed;
#[path = "flow_state.rs"]
mod flow_state;

use super::TaintSignal;
use super::tables::TaintTables;
use crate::review::diff::ChangedFile;
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
    let tainted = flow_seed::seed_tainted(root, content, tables);
    flow_checks::check_sinks(root, content, tables, file, &tainted, out);
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

#[cfg(test)]
mod tests {
    use super::super::sources::SourceKind;
    use super::flow_seed::has_nest_primitive_pipe;
    use super::flow_state::{TaintState, normalize_access_path};

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
        assert_eq!(normalize_access_path("items[12]"), Some("items.12".into()));
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
