use super::support::*;
use tempfile::tempdir;

#[test]
fn ref_range_resolution_uses_the_selected_head_file_inventory() {
    let temp = tempdir().expect("temp dir");
    let root = temp.path();
    init_repo(root);
    write(
        root,
        "src/api/index.ts",
        "export function loadUser() {}\nexport const marker = 1;\n",
    );
    write(
        root,
        "src/caller.ts",
        concat!(
            "import { loadUser } from \"./api\";\n",
            "import { marker } from \"./api/index.ts\";\n",
        ),
    );
    commit_all(root, "base");
    let base = git_stdout(root, &["rev-parse", "HEAD"]);

    write(root, "src/api/index.ts", "export const marker = 1;\n");
    commit_all(root, "selected head removes export");
    let selected_head = git_stdout(root, &["rev-parse", "HEAD"]);

    // The checkout graph resolves `./api` to the competing extension, but the
    // selected head contains only the index candidate. The explicit marker
    // import keeps the selected exporter in the graph's bounded caller set.
    write(root, "src/api.ts", "export function loadUser() {}\n");
    commit_all(root, "checkout-only competing extension");

    let report = run_review_json(
        root,
        &[
            "review",
            ".",
            "--base",
            &base,
            "--head",
            &selected_head,
            "--format",
            "json",
        ],
    );
    let signal = only_b21(&report);
    assert_eq!(signal["path"], "src/caller.ts");
    assert_eq!(signal["target_path"], "src/api/index.ts");
}

#[test]
fn ref_range_does_not_follow_checkout_only_index_resolution() {
    let temp = tempdir().expect("temp dir");
    let root = temp.path();
    init_repo(root);
    write(
        root,
        "src/api/index.ts",
        "export function loadUser() {}\nexport const marker = 1;\n",
    );
    write(root, "src/api.ts", "export function loadUser() {}\n");
    write(
        root,
        "src/caller.ts",
        "import { loadUser } from \"./api\";\n",
    );
    commit_all(root, "base");
    let base = git_stdout(root, &["rev-parse", "HEAD"]);

    write(root, "src/api/index.ts", "export const marker = 1;\n");
    commit_all(root, "selected head changes non-selected index");
    let selected_head = git_stdout(root, &["rev-parse", "HEAD"]);

    std::fs::remove_file(root.join("src/api.ts")).expect("remove competing extension");
    commit_all(root, "checkout resolves through index");

    let report = run_review_json(
        root,
        &[
            "review",
            ".",
            "--base",
            &base,
            "--head",
            &selected_head,
            "--format",
            "json",
        ],
    );
    assert!(b21_records(&report).is_empty(), "{report:#?}");
}
