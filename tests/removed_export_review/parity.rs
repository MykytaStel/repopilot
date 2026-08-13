use super::support::*;
use tempfile::tempdir;

#[test]
fn working_tree_ref_range_and_definitely_gate_share_the_canonical_occurrence() {
    let temp = tempdir().expect("temp dir");
    init_repo(temp.path());
    write(temp.path(), "src/api.ts", "export function loadUser() {}\n");
    write(
        temp.path(),
        "src/caller.ts",
        "import { loadUser } from \"./api.ts\";\n",
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
fn coordinated_rename_is_not_a_removed_export_failure() {
    let temp = tempdir().expect("temp dir");
    init_repo(temp.path());
    write(temp.path(), "src/api.ts", "export function loadUser() {}\n");
    write(
        temp.path(),
        "src/caller.ts",
        "import { loadUser } from \"./api.ts\";\nloadUser();\n",
    );
    commit_all(temp.path(), "before");
    write(
        temp.path(),
        "src/api.ts",
        "export function fetchAccount() {}\n",
    );
    write(
        temp.path(),
        "src/caller.ts",
        "import { fetchAccount } from \"./api.ts\";\nfetchAccount();\n",
    );

    let report = run_review_json(temp.path(), &["review", ".", "--format", "json"]);
    assert!(b21_records(&report).is_empty());
    let gated = run_review(
        temp.path(),
        &["review", ".", "--fail-on-review", "definitely"],
    );
    assert!(gated.status.success(), "{gated:?}");
}

#[test]
fn type_only_removed_export_is_reported_by_the_real_review_pipeline() {
    let temp = tempdir().expect("temp dir");
    init_repo(temp.path());
    write(temp.path(), "src/api.ts", "export type UserId = string;\n");
    write(
        temp.path(),
        "src/caller.ts",
        "import type { UserId } from \"./api.ts\";\n",
    );
    commit_all(temp.path(), "before");
    write(
        temp.path(),
        "src/api.ts",
        "export type AccountId = string;\n",
    );

    let report = run_review_json(temp.path(), &["review", ".", "--format", "json"]);
    let signal = only_b21(&report);
    assert!(
        signal["detail"]
            .as_str()
            .is_some_and(|detail| detail.contains("Removed type export 'UserId'")),
        "{signal:#?}"
    );
}

#[test]
fn parse_failure_is_suppressed_by_the_real_review_pipeline() {
    let temp = tempdir().expect("temp dir");
    init_repo(temp.path());
    write(temp.path(), "src/api.ts", "export function loadUser() {}\n");
    write(
        temp.path(),
        "src/caller.ts",
        "import { loadUser } from \"./api.ts\";\nconst broken = ;\n",
    );
    commit_all(temp.path(), "before");
    write(
        temp.path(),
        "src/api.ts",
        "export function saveUserAccount() {}\n",
    );

    let report = run_review_json(temp.path(), &["review", ".", "--format", "json"]);
    assert!(b21_records(&report).is_empty(), "{report:#?}");
}
