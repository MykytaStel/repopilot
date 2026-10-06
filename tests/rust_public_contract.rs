#[allow(dead_code)]
#[path = "removed_export_review/support.rs"]
mod review;
#[allow(dead_code)]
#[path = "removed_export_changed_scan/support.rs"]
mod scan;
use serde_json::Value;

#[test]
fn rust_contract_cache_scopes_and_current_source_reconstruction() {
    let temp = tempfile::tempdir().unwrap();
    let root = temp.path();
    review::init_repo(root);
    review::write(root, "src/api.rs", "pub fn load() {}\n");
    review::write(
        root,
        "src/lib.rs",
        "mod api;\nuse self::api::load;\nfn run() { load(); }\n",
    );
    review::commit_all(root, "before");
    review::write(root, "src/api.rs", "pub fn save() {}\n");
    let cold = review::run_review_json(root, &["review", ".", "--format", "json"]);
    let warm = review::run_review_json(root, &["review", ".", "--format", "json"]);
    let signal = review::only_b21(&cold);
    assert_eq!(signal["path"], "src/lib.rs");
    assert_eq!(signal["target_path"], "src/api.rs");
    assert_eq!(
        signal["headline"],
        "removed public function is still referenced"
    );
    assert_eq!(signal, review::only_b21(&warm));
    let full = scan::scan_json(root, &[]);
    scan::assert_rule_absent(&full);
    let changed = scan::scan_json(root, &["--changed"]);
    let finding = scan::finding_for_rule(&changed).clone();
    assert_eq!(finding["evidence"][0]["path"], "src/lib.rs");
    assert_eq!(finding["evidence"][0]["line_start"], 2);
    assert_eq!(
        &finding,
        scan::finding_for_rule(&scan::scan_json(root, &["--changed"]))
    );
    scan::remove_caller_symbol_facts(root);
    assert_eq!(
        &finding,
        scan::finding_for_rule(&scan::scan_json(root, &["--changed"]))
    );
    invalidate_pre_rust_cache(root);
    assert_eq!(
        &finding,
        scan::finding_for_rule(&scan::scan_json(root, &["--changed"]))
    );
    review::write(
        root,
        "src/lib.rs",
        "mod api; use self::api::save; fn run() { save(); }\n",
    );
    scan::assert_rule_absent(&scan::scan_json(root, &["--changed"]));
    let safe = review::run_review_json(root, &["review", ".", "--format", "json"]);
    assert!(review::b21_records(&safe).is_empty());
}

fn invalidate_pre_rust_cache(root: &std::path::Path) {
    let path = root.join(".repopilot/cache/parsed_facts_v2.json");
    let mut cache: Value = serde_json::from_slice(&std::fs::read(&path).unwrap()).unwrap();
    cache["schema_version"] = 7.into();
    cache["analysis_version"] =
        "tree-sitter-imports-exports-default-symbols-spans-guarded-syntax-v6".into();
    std::fs::write(path, serde_json::to_vec(&cache).unwrap()).unwrap();
}

#[test]
fn rust_same_line_qualified_calls_keep_occurrence_identity() {
    let temp = tempfile::tempdir().unwrap();
    let root = temp.path();
    review::init_repo(root);
    review::write(root, "src/api/mod.rs", "pub fn load() {}\n");
    review::write(
        root,
        "src/lib.rs",
        "mod api; fn run() { api::load(); self::api::load(); }\n",
    );
    review::commit_all(root, "before");
    review::write(root, "src/api/mod.rs", "pub fn save() {}\n");
    let report = review::run_review_json(root, &["review", ".", "--format", "json"]);
    let signals = review::b21_records(&report);
    assert_eq!(signals.len(), 2);
    assert_ne!(signals[0]["signal_id"], signals[1]["signal_id"]);
    let report = scan::scan_json(root, &["--changed"]);
    let findings = report["findings"]
        .as_array()
        .unwrap()
        .iter()
        .filter(|finding| finding["rule_id"] == scan::RULE_ID)
        .collect::<Vec<_>>();
    assert_eq!(findings.len(), 2);
    assert_ne!(
        findings[0]["evidence"][0]["snippet"],
        findings[1]["evidence"][0]["snippet"]
    );
    let warm = scan::scan_json(root, &["--changed"]);
    let warm_findings = warm["findings"]
        .as_array()
        .unwrap()
        .iter()
        .filter(|finding| finding["rule_id"] == scan::RULE_ID)
        .collect::<Vec<_>>();
    assert_eq!(findings, warm_findings);
    std::fs::remove_file(root.join("src/lib.rs")).unwrap();
    scan::assert_rule_absent(&scan::scan_json(root, &["--changed"]));
}

#[test]
fn rust_shadowed_calls_are_safe_across_cold_warm_and_previous_rust_cache() {
    let temp = tempfile::tempdir().unwrap();
    let root = temp.path();
    review::init_repo(root);
    review::write(root, "src/api.rs", "pub fn load() {}\n");
    review::write(
        root,
        "src/lib.rs",
        "mod api; fn run() { struct api; impl api { fn load() {} } api::load(); }\n",
    );
    review::commit_all(root, "before");
    review::write(root, "src/api.rs", "pub fn save() {}\n");
    for _ in 0..2 {
        let report = review::run_review_json(root, &["review", ".", "--format", "json"]);
        assert!(review::b21_records(&report).is_empty());
        scan::assert_rule_absent(&scan::scan_json(root, &["--changed"]));
    }
    let path = root.join(".repopilot/cache/parsed_facts_v2.json");
    let mut cache: Value = serde_json::from_slice(&std::fs::read(&path).unwrap()).unwrap();
    cache["schema_version"] = 8.into();
    cache["analysis_version"] =
        "tree-sitter-imports-exports-default-rust-symbols-spans-guarded-syntax-v7".into();
    for entry in cache["entries"].as_array_mut().unwrap() {
        if entry["javascript_symbols"]["exports"]
            .as_array()
            .is_some_and(Vec::is_empty)
        {
            entry["javascript_symbols"]["imports"] = serde_json::json!([{
                "imported_name":"load", "local_name":"load", "kind":"Value", "module_specifier":"mod::api",
                "line_start":1, "line_end":1, "byte_start":72, "byte_end":81
            }]);
        }
    }
    std::fs::write(&path, serde_json::to_vec(&cache).unwrap()).unwrap();
    scan::assert_rule_absent(&scan::scan_json(root, &["--changed"]));
    let rebuilt: Value = serde_json::from_slice(&std::fs::read(path).unwrap()).unwrap();
    assert_eq!(rebuilt["schema_version"], 9);
    review::write(
        root,
        "src/lib.rs",
        "mod api; fn run() { struct api; self::api::load(); }\n",
    );
    scan::finding_for_rule(&scan::scan_json(root, &["--changed"]));
}
