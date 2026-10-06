#[path = "removed_export_changed_scan/support.rs"]
mod support;
use support::*;

#[test]
fn changed_scan_reports_removed_export_on_the_surviving_caller_import() {
    let temp = tempfile::tempdir().expect("temp dir");
    let root = temp.path();
    init_repo(root);
    write(root, "src/api.ts", "export function loadUser() {}\n");
    write(
        root,
        "src/caller.ts",
        "import { loadUser as load } from './api.ts';\nload();\n",
    );
    commit_all(root, "before");
    write(root, "src/api.ts", "export function saveUser() {}\n");

    let report = scan_json(root, &["--changed"]);
    let finding = report["findings"]
        .as_array()
        .into_iter()
        .flatten()
        .find(|finding| finding["rule_id"] == RULE_ID)
        .unwrap_or_else(|| panic!("missing {RULE_ID}: {report:#?}"));

    assert_eq!(finding["severity"], "HIGH");
    assert_eq!(finding["confidence"], "HIGH");
    assert_eq!(finding["provenance"]["analysis_scope"], "git-diff");
    assert_eq!(finding["evidence"][0]["path"], "src/caller.ts");
    assert_eq!(finding["evidence"][0]["line_start"], 1);
    assert!(
        finding["evidence"][0]["snippet"]
            .as_str()
            .is_some_and(|snippet| snippet.contains("src/api.ts"))
    );
}

#[test]
fn cold_and_warm_changed_scans_keep_identical_removed_export_identity() {
    let temp = tempfile::tempdir().expect("temp dir");
    let root = temp.path();
    init_repo(root);
    write(root, "src/api.ts", "export function loadUser() {}\n");
    write(
        root,
        "src/caller.ts",
        "import { loadUser } from './api.ts';\nloadUser();\n",
    );
    commit_all(root, "before");
    write(root, "src/api.ts", "export function saveUser() {}\n");

    assert_rule_absent(&scan_json(root, &[]));
    let cold = scan_json(root, &["--changed"]);
    let warm = scan_json(root, &["--changed"]);

    assert_eq!(finding_for_rule(&cold), finding_for_rule(&warm));
    assert_eq!(warm["context_graph_cache"]["status"], "hit");
    assert!(
        warm["cache_telemetry"]["parsed_cache_hits"]
            .as_u64()
            .is_some_and(|hits| hits > 0)
    );
}

#[test]
fn coordinated_export_and_caller_rename_stays_silent() {
    let temp = tempfile::tempdir().expect("temp dir");
    let root = temp.path();
    init_repo(root);
    write(root, "src/api.ts", "export function loadUser() {}\n");
    write(
        root,
        "src/caller.ts",
        "import { loadUser } from './api.ts';\nloadUser();\n",
    );
    commit_all(root, "before");
    write(root, "src/api.ts", "export function saveUser() {}\n");
    write(
        root,
        "src/caller.ts",
        "import { saveUser } from './api.ts';\nsaveUser();\n",
    );

    let report = scan_json(root, &["--changed"]);

    assert_rule_absent(&report);
}

#[test]
fn since_base_uses_the_selected_pre_change_exporter() {
    let temp = tempfile::tempdir().expect("temp dir");
    let root = temp.path();
    init_repo(root);
    write(root, "src/api.ts", "export function loadUser() {}\n");
    write(
        root,
        "src/caller.ts",
        "import { loadUser } from './api.ts';\nloadUser();\n",
    );
    commit_all(root, "before");
    let base = git_stdout(root, &["rev-parse", "HEAD"]);
    write(root, "src/api.ts", "export function saveUser() {}\n");
    commit_all(root, "after");

    let report = scan_json(root, &["--since", &base]);

    assert_eq!(
        finding_for_rule(&report)["evidence"][0]["path"],
        "src/caller.ts"
    );
}

#[test]
fn missing_current_caller_facts_rebuild_without_losing_occurrence() {
    let temp = tempfile::tempdir().expect("temp dir");
    let root = temp.path();
    init_repo(root);
    write(root, "src/api.ts", "export function loadUser() {}\n");
    write(
        root,
        "src/caller.ts",
        "import { loadUser } from './api.ts';\nloadUser();\n",
    );
    commit_all(root, "before");
    write(root, "src/api.ts", "export function saveUser() {}\n");
    let cold = scan_json(root, &["--changed"]);
    finding_for_rule(&cold);
    remove_caller_symbol_facts(root);

    let missing = scan_json(root, &["--changed"]);

    assert_eq!(finding_for_rule(&cold), finding_for_rule(&missing));
    assert_eq!(missing["context_graph_cache"]["status"], "hit");
    assert!(
        missing["cache_telemetry"]["parsed_cache_misses"]
            .as_u64()
            .is_some_and(|misses| misses > 0)
    );
}

#[test]
fn full_scan_without_a_comparison_revision_stays_silent() {
    let temp = tempfile::tempdir().expect("temp dir");
    let root = temp.path();
    init_repo(root);
    write(root, "src/api.ts", "export function loadUser() {}\n");
    write(
        root,
        "src/caller.ts",
        "import { loadUser } from './api.ts';\nloadUser();\n",
    );
    commit_all(root, "before");
    write(root, "src/api.ts", "export function saveUser() {}\n");

    let report = scan_json(root, &[]);

    assert_rule_absent(&report);
}

#[test]
fn shared_rule_id_obeys_disable_and_severity_override() {
    let temp = tempfile::tempdir().expect("temp dir");
    let root = temp.path();
    init_repo(root);
    write(root, "src/api.ts", "export function loadUser() {}\n");
    write(
        root,
        "src/caller.ts",
        "import { loadUser } from './api.ts';\nloadUser();\n",
    );
    commit_all(root, "before");
    write(root, "src/api.ts", "export function saveUser() {}\n");
    write(
        root,
        "repopilot.toml",
        "[rules]\ndisable = [\"behavioral.removed-export-still-imported\"]\n",
    );

    assert_rule_absent(&scan_json(root, &["--changed"]));

    write(
        root,
        "repopilot.toml",
        "[rules.severity_overrides]\n\"behavioral.removed-export-still-imported\" = \"low\"\n",
    );
    let overridden = scan_json(root, &["--changed", "--profile", "strict"]);
    assert_eq!(finding_for_rule(&overridden)["severity"], "LOW");
}

#[test]
fn same_line_aliases_keep_distinct_scan_occurrences() {
    let temp = tempfile::tempdir().expect("temp dir");
    let root = temp.path();
    init_repo(root);
    write(root, "src/api.ts", "export function loadUser() {}\n");
    write(
        root,
        "src/caller.ts",
        "import { loadUser as first, loadUser as second } from './api.ts';\n",
    );
    commit_all(root, "before");
    write(root, "src/api.ts", "export function saveUser() {}\n");

    let report = scan_json(root, &["--changed"]);
    let findings = report["findings"]
        .as_array()
        .expect("findings")
        .iter()
        .filter(|finding| finding["rule_id"] == RULE_ID)
        .collect::<Vec<_>>();
    let occurrence_keys = findings
        .iter()
        .map(|finding| finding["occurrence_key"].as_str().expect("occurrence key"))
        .collect::<std::collections::BTreeSet<_>>();

    assert_eq!(findings.len(), 2);
    assert_eq!(occurrence_keys.len(), 2);
}

#[test]
fn default_export_cold_warm_and_full_scan_scope() {
    let temp = tempfile::tempdir().expect("temp dir");
    let root = temp.path();
    init_repo(root);
    write(
        root,
        "src/api.ts",
        "export default function loadUser() {}\n",
    );
    write(
        root,
        "src/caller.ts",
        "import loadUser from './api.ts';\nloadUser();\n",
    );
    commit_all(root, "before");
    write(root, "src/api.ts", "export function saveUser() {}\n");

    assert_rule_absent(&scan_json(root, &[]));
    let cold = scan_json(root, &["--changed"]);
    let warm = scan_json(root, &["--changed"]);

    assert_eq!(finding_for_rule(&cold), finding_for_rule(&warm));
    assert_eq!(warm["context_graph_cache"]["status"], "hit");
    assert!(
        warm["cache_telemetry"]["parsed_cache_hits"]
            .as_u64()
            .is_some_and(|hits| hits > 0)
    );
}

#[test]
fn default_export_legacy_cache_rebuilds_missing_symbol_facts() {
    let temp = tempfile::tempdir().unwrap();
    let root = temp.path();
    init_repo(root);
    write(root, "src/api.ts", "export default 42;\n");
    write(root, "src/caller.ts", "import load from './api.ts';\n");
    commit_all(root, "before");
    write(root, "src/api.ts", "export const value = 42;\n");
    let cold = scan_json(root, &["--changed"]);
    let path = root.join(".repopilot/cache/parsed_facts_v2.json");
    let mut cache: serde_json::Value =
        serde_json::from_slice(&std::fs::read(&path).unwrap()).unwrap();
    cache["schema_version"] = serde_json::json!(6);
    for entry in cache["entries"].as_array_mut().unwrap() {
        if let Some(imports) = entry["javascript_symbols"]["imports"].as_array_mut() {
            imports.retain(|import| import["imported_name"] != "default");
        }
    }
    std::fs::write(&path, serde_json::to_vec(&cache).unwrap()).unwrap();
    let rebuilt = scan_json(root, &["--changed"]);
    assert_eq!(finding_for_rule(&cold), finding_for_rule(&rebuilt));
    let cache: serde_json::Value = serde_json::from_slice(&std::fs::read(path).unwrap()).unwrap();
    assert_eq!(cache["schema_version"], 10);
}

#[path = "removed_export_changed_scan/source_scope.rs"]
mod source_scope;
