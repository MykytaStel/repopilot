use super::support::*;
use std::collections::BTreeSet;
use tempfile::tempdir;

#[test]
fn occurrence_ids_are_distinct_and_repeated_output_is_stably_sorted() {
    let temp = tempdir().expect("temp dir");
    init_repo(temp.path());
    write(
        temp.path(),
        "src/api.ts",
        "export function loadUser() {}\nexport function saveUser() {}\n",
    );
    write(
        temp.path(),
        "src/caller_one.ts",
        "import { loadUser, saveUser } from \"./api.ts\";\n",
    );
    write(
        temp.path(),
        "src/caller_two.ts",
        "import { loadUser } from \"./api.ts\";\n",
    );
    commit_all(temp.path(), "before");
    write(
        temp.path(),
        "src/api.ts",
        "export function listAccounts() {}\n",
    );

    let first = run_review_json(temp.path(), &["review", ".", "--format", "json"]);
    let second = run_review_json(temp.path(), &["review", ".", "--format", "json"]);
    let first_records = b21_records(&first);
    assert_eq!(first_records.len(), 3, "{first_records:#?}");
    assert_eq!(first_records, b21_records(&second));

    let ids = first_records
        .iter()
        .map(|signal| signal["signal_id"].as_str().expect("signal id"))
        .collect::<BTreeSet<_>>();
    assert_eq!(ids.len(), 3, "each import occurrence owns a stable id");
    let paths = first_records
        .iter()
        .map(|signal| signal["path"].as_str().expect("caller path"))
        .collect::<Vec<_>>();
    assert_eq!(
        paths,
        [
            "src/caller_one.ts",
            "src/caller_one.ts",
            "src/caller_two.ts"
        ]
    );
    let details = first_records
        .iter()
        .filter(|signal| signal["path"] == "src/caller_one.ts")
        .map(|signal| signal["detail"].as_str().expect("signal detail"))
        .collect::<Vec<_>>();
    assert!(details.iter().any(|detail| detail.contains("loadUser")));
    assert!(details.iter().any(|detail| detail.contains("saveUser")));
}

#[test]
fn same_symbol_aliases_on_one_line_keep_distinct_occurrence_ids() {
    let temp = tempdir().expect("temp dir");
    init_repo(temp.path());
    write(temp.path(), "src/api.ts", "export function loadUser() {}\n");
    write(
        temp.path(),
        "src/caller.ts",
        "import { loadUser as first, loadUser as second } from \"./api.ts\";\n",
    );
    commit_all(temp.path(), "before");
    write(
        temp.path(),
        "src/api.ts",
        "export function saveUserAccount() {}\n",
    );

    let report = run_review_json(temp.path(), &["review", ".", "--format", "json"]);
    let records = b21_records(&report);
    assert_eq!(records.len(), 2, "{records:#?}");
    let ids = records
        .iter()
        .map(|signal| signal["signal_id"].as_str().expect("signal id"))
        .collect::<BTreeSet<_>>();
    assert_eq!(ids.len(), 2, "each AST occurrence needs its own stable id");
    let details = records
        .iter()
        .map(|signal| signal["detail"].as_str().expect("signal detail"))
        .collect::<Vec<_>>();
    assert!(details.iter().any(|detail| detail.contains("'first'")));
    assert!(details.iter().any(|detail| detail.contains("'second'")));
}
