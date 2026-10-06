use super::support::*;
use tempfile::tempdir;

#[test]
fn default_export_working_tree_and_refs_share_exact_occurrence() {
    let temp = tempdir().expect("temp dir");
    init_repo(temp.path());
    write(
        temp.path(),
        "src/api.ts",
        "export default function loadUser() {}\n",
    );
    write(
        temp.path(),
        "src/caller.ts",
        "import loadUser from \"./api.ts\";\n",
    );
    commit_all(temp.path(), "before");
    let snapshot = run_review(temp.path(), &["snapshot"]);
    assert!(snapshot.status.success(), "{snapshot:?}");
    write(
        temp.path(),
        "src/api.ts",
        "export function saveUserAccount() {}\n",
    );

    let working = run_review_json(temp.path(), &["review", ".", "--format", "json"]);
    let working_signal = only_b21(&working);
    assert_canonical_occurrence(working_signal);
    assert_eq!(working["change_proof"]["verdict"], "BROKEN");
    assert_eq!(
        working["change_proof"]["contract_deltas"][0]["family"],
        "public-symbol"
    );
    assert_eq!(
        working["change_proof"]["contract_deltas"][0]["change"],
        "removed-export"
    );
    assert_eq!(
        working["change_proof"]["contract_deltas"][0]["exporter_path"],
        "src/api.ts"
    );
    assert_eq!(
        working["change_proof"]["contract_deltas"][0]["consumer_path"],
        "src/caller.ts"
    );
    let snapshot_style = run_review_json(
        temp.path(),
        &["review", ".", "--since-snapshot", "--format", "json"],
    );
    assert_eq!(working_signal, only_b21(&snapshot_style));

    let gated = run_review(
        temp.path(),
        &["review", ".", "--fail-on-review", "definitely"],
    );
    assert_eq!(gated.status.code(), Some(1), "{gated:?}");
    assert!(String::from_utf8_lossy(&gated.stderr).contains("review gate failed"));

    commit_all(temp.path(), "after");
    let refs = run_review_json(
        temp.path(),
        &[
            "review", ".", "--base", "HEAD~1", "--head", "HEAD", "--format", "json",
        ],
    );
    let ref_signal = only_b21(&refs);
    assert_canonical_occurrence(ref_signal);
    assert_eq!(working_signal, ref_signal);
}

#[test]
fn default_export_coordinated_or_removed_caller_is_safe() {
    for caller in [Some("import { load } from './api.ts';\n"), None] {
        let temp = tempdir().unwrap();
        let root = temp.path();
        init_repo(root);
        write(root, "src/api.ts", "export default function load() {}\n");
        write(root, "src/caller.ts", "import load from './api.ts';\n");
        commit_all(root, "before");
        write(root, "src/api.ts", "export function load() {}\n");
        match caller {
            Some(content) => write(root, "src/caller.ts", content),
            None => std::fs::remove_file(root.join("src/caller.ts")).unwrap(),
        }
        let report = run_review_json(root, &["review", ".", "--format", "json"]);
        assert!(b21_records(&report).is_empty());
    }
}
