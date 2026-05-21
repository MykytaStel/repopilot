use super::*;

#[test]
fn repopilot_internal_paths_are_excluded_from_review_scope() {
    assert!(is_repopilot_internal_path(Path::new(
        ".repopilot/cache/repo_context.json"
    )));
    assert!(is_repopilot_internal_path(Path::new(
        r".repopilot\cache\repo_context.json"
    )));
    assert!(!is_repopilot_internal_path(Path::new("src/lib.rs")));
}
