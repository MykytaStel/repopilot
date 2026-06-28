use repopilot::scan::cache::CACHE_SCHEMA_VERSION;
use serde_json::Value;
use std::fs;
use std::path::Path;
use std::process::Command;
use tempfile::tempdir;

fn repopilot() -> Command {
    Command::new(env!("CARGO_BIN_EXE_repopilot"))
}

#[test]
fn changed_scan_writes_cache_and_reuses_matching_findings() {
    let temp = tempdir().expect("temp dir");
    init_repo(temp.path());
    write(temp.path().join("src/lib.rs"), "pub fn live() {}\n");
    commit_all(temp.path(), "initial");

    write(
        temp.path().join("src/lib.rs"),
        "pub fn live() {}\nconst API_KEY: &str = \"abc123xyz987\";\n",
    );

    let first = scan_changed_json(temp.path(), &["--changed"]);
    assert_eq!(first["mode"], "changed");
    assert_eq!(first["repo_level_rules_included"], true);
    assert_eq!(first["changed_files_count"], 1);
    assert_eq!(first["cache_telemetry"]["hits"], 0);
    assert_eq!(first["cache_telemetry"]["misses"], 1);
    assert_eq!(first["cache_telemetry"]["skipped"], 0);
    assert_eq!(
        first["cache_telemetry"]["changed_files"][0]["path"],
        "src/lib.rs"
    );
    assert_eq!(
        first["cache_telemetry"]["changed_files"][0]["change_reason"],
        "modified"
    );
    assert_eq!(
        first["cache_telemetry"]["changed_files"][0]["cache_status"],
        "miss"
    );
    assert_eq!(
        first["cache_telemetry"]["changed_files"][0]["cache_reason"],
        "missing-cache-entry"
    );
    assert_changed_reason(&first, "modified", 1);
    assert!(
        first["cache_telemetry"]["timings"]["miss_scan_us"]
            .as_u64()
            .is_some()
    );
    assert_rule_present(&first, "security.secret-candidate");
    assert_eq!(first["context_graph_cache"]["status"], "miss");
    assert!(first["context_graph_summary"]["files"].as_u64().is_some());

    let cache_dir = temp.path().join(".repopilot/cache");
    assert!(cache_dir.join("file_hashes.json").is_file());
    assert!(cache_dir.join("file_roles.json").is_file());
    assert!(cache_dir.join("findings.json").is_file());
    assert!(cache_dir.join("repo_context.json").is_file());
    assert_eq!(
        read_json(&cache_dir.join("file_hashes.json"))["schema_version"],
        CACHE_SCHEMA_VERSION
    );
    assert_eq!(
        read_json(&cache_dir.join("file_hashes.json"))["entries"][0]["hash"]
            .as_str()
            .expect("hash should be string")
            .len(),
        64
    );

    rewrite_cached_finding_title(&cache_dir.join("findings.json"), "Cached secret title");
    let cached = scan_changed_json(temp.path(), &["--changed"]);
    assert_eq!(cached["cache_telemetry"]["hits"], 1);
    assert_eq!(cached["context_graph_cache"]["status"], "hit");
    assert_eq!(cached["cache_telemetry"]["misses"], 0);
    assert_eq!(cached["cache_telemetry"]["hit_rate_percent"], 100);
    assert_eq!(
        cached["cache_telemetry"]["changed_files"][0]["cache_status"],
        "hit"
    );
    assert_eq!(
        cached["cache_telemetry"]["changed_files"][0]["cache_reason"],
        "unchanged-content-and-config"
    );
    assert_eq!(first_finding_title(&cached), "Cached secret title");

    write(
        temp.path().join("src/lib.rs"),
        "pub fn live() {}\nconst API_KEY: &str = \"xyz987abc123\";\n",
    );
    let invalidated = scan_changed_json(temp.path(), &["--changed"]);
    assert_eq!(invalidated["cache_telemetry"]["hits"], 0);
    assert_eq!(invalidated["cache_telemetry"]["misses"], 1);
    assert_eq!(
        invalidated["cache_telemetry"]["changed_files"][0]["cache_reason"],
        "content-changed"
    );
    assert_eq!(
        first_finding_title(&invalidated),
        "Possible secret detected"
    );
}

#[test]
fn changed_scan_and_cache_keep_distinct_secret_occurrences_with_colliding_baseline_keys() {
    let temp = tempdir().expect("temp dir");
    init_repo(temp.path());
    write(temp.path().join("src/config.py"), "print('clean')\n");
    commit_all(temp.path(), "initial");
    write(
        temp.path().join("src/config.py"),
        "API_KEY = \"abc123xyz987\"\nAPI_KEY = \"def456uvw654\"\n",
    );

    let first = scan_changed_json(temp.path(), &["--changed", "--profile", "strict"]);
    assert_eq!(secret_candidate_count(&first), 2);
    assert_eq!(first["cache_telemetry"]["hits"], 0);
    assert_eq!(first["cache_telemetry"]["misses"], 1);

    let cached = scan_changed_json(temp.path(), &["--changed", "--profile", "strict"]);
    assert_eq!(secret_candidate_count(&cached), 2);
    assert_eq!(cached["cache_telemetry"]["hits"], 1);
    assert_eq!(cached["cache_telemetry"]["misses"], 0);
}

#[test]
fn changed_scan_with_no_changes_skips_repo_context_and_cache_write() {
    let temp = tempdir().expect("temp dir");
    init_repo(temp.path());
    write(temp.path().join("src/lib.rs"), "pub fn live() {}\n");
    commit_all(temp.path(), "initial");

    let json = scan_changed_json(temp.path(), &["--changed", "--timing"]);

    assert_eq!(json["mode"], "changed");
    assert_eq!(json["changed_files_count"], 0);
    assert!(json.get("cache_telemetry").is_none());
    assert!(json.get("context_graph_summary").is_none());
    assert!(json.get("context_graph_cache").is_none());
    assert_eq!(json["scan_timings"]["framework_detection_us"], 0);
    assert!(!temp.path().join(".repopilot/cache").exists());
}

#[test]
fn changed_scan_invalidates_cache_when_config_changes() {
    let temp = tempdir().expect("temp dir");
    init_repo(temp.path());
    write(temp.path().join("src/lib.rs"), "pub fn live() {}\n");
    commit_all(temp.path(), "initial");
    write(
        temp.path().join("src/lib.rs"),
        "pub fn live() {}\nconst API_KEY: &str = \"abc123xyz987\";\n",
    );

    scan_changed_json(temp.path(), &["--changed"]);
    let changed_config = scan_changed_json(temp.path(), &["--changed", "--max-file-loc", "42"]);

    assert_eq!(changed_config["cache_telemetry"]["hits"], 0);
    assert_eq!(changed_config["cache_telemetry"]["misses"], 1);
    assert_eq!(changed_config["context_graph_cache"]["status"], "miss");
    assert_eq!(
        changed_config["cache_telemetry"]["changed_files"][0]["cache_reason"],
        "config-changed"
    );
}

#[test]
fn changed_scan_invalidates_old_cache_schema() {
    let temp = tempdir().expect("temp dir");
    init_repo(temp.path());
    write(temp.path().join("src/lib.rs"), "pub fn live() {}\n");
    commit_all(temp.path(), "initial");
    write(
        temp.path().join("src/lib.rs"),
        "pub fn live() {}\nconst API_KEY: &str = \"abc123xyz987\";\n",
    );

    scan_changed_json(temp.path(), &["--changed"]);
    for name in ["file_hashes.json", "file_roles.json", "findings.json"] {
        force_cache_schema(temp.path().join(".repopilot/cache").join(name).as_path(), 5);
    }
    let json = scan_changed_json(temp.path(), &["--changed"]);

    assert_eq!(json["cache_telemetry"]["hits"], 0);
    assert_eq!(json["cache_telemetry"]["misses"], 1);
    assert_eq!(
        json["cache_telemetry"]["changed_files"][0]["cache_reason"],
        "missing-cache-entry"
    );
}

#[test]
fn changed_scan_rejects_v5_cache_missing_build_tooling_roles() {
    let temp = tempdir().expect("temp dir");
    init_repo(temp.path());
    write(
        temp.path()
            .join("buildSrc/src/main/kotlin/com/app/BuildPlugin.kt"),
        "fun configure() = Unit\n",
    );
    commit_all(temp.path(), "initial");
    write(
        temp.path()
            .join("buildSrc/src/main/kotlin/com/app/BuildPlugin.kt"),
        "fun configure(): Unit = TODO()\n",
    );

    let first_changed = scan_changed_json(temp.path(), &["--changed", "--profile", "strict"]);
    assert_eq!(
        first_changed["cache_telemetry"]["changed_files"][0]["cache_status"],
        "miss"
    );
    assert_eq!(
        finding_for_rule(&first_changed, "language.managed.fatal-exception-risk")["severity"],
        "LOW"
    );

    let cache_dir = temp.path().join(".repopilot/cache");
    for name in ["file_hashes.json", "file_roles.json", "findings.json"] {
        force_cache_schema(cache_dir.join(name).as_path(), 5);
    }
    remove_cached_role(&cache_dir.join("file_roles.json"), "build-tooling");
    set_cached_rule_severity(
        &cache_dir.join("findings.json"),
        "language.managed.fatal-exception-risk",
        "HIGH",
    );

    let upgraded_changed = scan_changed_json(temp.path(), &["--changed", "--profile", "strict"]);
    assert_eq!(
        upgraded_changed["cache_telemetry"]["changed_files"][0]["cache_status"], "miss",
        "v5 file-role caches must be rejected after role classification changed: {upgraded_changed:#?}"
    );
    assert_eq!(
        finding_for_rule(&upgraded_changed, "language.managed.fatal-exception-risk")["severity"],
        "LOW",
        "re-analysis must restore build-tooling downgrade: {upgraded_changed:#?}"
    );
}

#[test]
fn changed_scan_uses_repo_graph_context_for_changed_findings_only() {
    let temp = tempdir().expect("temp dir");
    init_repo(temp.path());
    write(temp.path().join("src/shared.rs"), "pub fn shared() {}\n");
    write(
        temp.path().join("src/a.rs"),
        "use crate::shared;\npub fn a() {}\n",
    );
    write(
        temp.path().join("src/b.rs"),
        "use crate::shared;\npub fn b() {}\n",
    );
    commit_all(temp.path(), "initial");
    write(
        temp.path().join("src/shared.rs"),
        "pub fn shared() {}\nconst API_KEY: &str = \"abc123xyz987\";\n",
    );

    let json = scan_changed_json(temp.path(), &["--changed"]);
    let secret = json["findings"]
        .as_array()
        .expect("findings array")
        .iter()
        .find(|finding| finding["rule_id"] == "security.secret-candidate")
        .expect("secret finding");

    assert!(
        secret["risk"]["signals"]
            .as_array()
            .expect("risk signals")
            .iter()
            .any(|signal| signal["id"] == "graph.dependency")
    );
    assert!(
        json["findings"]
            .as_array()
            .expect("findings array")
            .iter()
            .all(|finding| finding["evidence"][0]["path"] == "src/shared.rs")
    );
}

#[test]
fn changed_scan_cache_hit_keeps_changed_file_imports_for_graph_patch() {
    let temp = tempdir().expect("temp dir");
    init_repo(temp.path());
    write(temp.path().join("src/old.rs"), "pub fn old() {}\n");
    write(temp.path().join("src/new.rs"), "pub fn new() {}\n");
    write(
        temp.path().join("src/lib.rs"),
        "use crate::old;\npub fn live() { old::old(); }\n",
    );
    commit_all(temp.path(), "initial");

    let initial_graph = scan_graph_json(temp.path());
    assert_eq!(initial_graph["context_graph_cache"]["status"], "write");
    let stale_graph_cache =
        fs::read(temp.path().join(".repopilot/cache/repo_context.json")).expect("read graph cache");

    write(
        temp.path().join("src/lib.rs"),
        "use crate::new;\npub fn live() { new::new(); }\nconst API_KEY: &str = \"abc123xyz987\";\n",
    );
    let first_changed = scan_changed_json(temp.path(), &["--changed"]);
    assert_eq!(
        first_changed["cache_telemetry"]["changed_files"][0]["cache_status"],
        "miss"
    );

    fs::write(
        temp.path().join(".repopilot/cache/repo_context.json"),
        stale_graph_cache,
    )
    .expect("restore stale graph cache");

    let cached_changed = scan_changed_json(temp.path(), &["--changed"]);
    assert_eq!(
        cached_changed["cache_telemetry"]["changed_files"][0]["cache_status"],
        "hit"
    );
    assert!(
        top_dependencies_include(&cached_changed, "src/new.rs"),
        "cached changed file imports should patch stale graph cache: {cached_changed:#?}"
    );
}

#[test]
fn changed_scan_cache_hit_preserves_deferred_imports_for_graph_patch() {
    let temp = tempdir().expect("temp dir");
    init_repo(temp.path());
    write(temp.path().join("app/__init__.py"), "");
    write(temp.path().join("app/models.py"), "thing = object()\n");
    write(
        temp.path().join("app/views.py"),
        "def handler():\n    return None\n",
    );
    commit_all(temp.path(), "initial");

    let initial_graph = scan_graph_json(temp.path());
    assert_eq!(initial_graph["context_graph_cache"]["status"], "write");
    let stale_graph_cache =
        fs::read(temp.path().join(".repopilot/cache/repo_context.json")).expect("read graph cache");

    write(
        temp.path().join("app/models.py"),
        "import app.views\nthing = object()\nAPI_KEY = \"abc123xyz987\"\n",
    );
    write(
        temp.path().join("app/views.py"),
        "def handler():\n    from app.models import thing\n    return thing\n",
    );
    let first_changed = scan_changed_json(temp.path(), &["--changed"]);
    assert_eq!(
        first_changed["cache_telemetry"]["changed_files"][0]["cache_status"],
        "miss"
    );

    fs::write(
        temp.path().join(".repopilot/cache/repo_context.json"),
        stale_graph_cache,
    )
    .expect("restore stale graph cache");

    let cached_changed = scan_changed_json(temp.path(), &["--changed"]);
    assert_eq!(
        cached_changed["cache_telemetry"]["changed_files"][0]["cache_status"],
        "hit"
    );
    let cycles = &cached_changed["context_graph_summary"]["cycles"];
    assert!(
        cycles.as_array().is_none_or(Vec::is_empty),
        "deferred imports from cache hits must not become eager cycles: {cached_changed:#?}"
    );
}

#[test]
fn changed_scan_rejects_v4_cache_missing_inline_type_only_deferred_imports() {
    let temp = tempdir().expect("temp dir");
    init_repo(temp.path());
    write(
        temp.path().join("src/a.ts"),
        "import { type B } from \"./b\";\nexport type A = { b?: B };\n",
    );
    write(
        temp.path().join("src/b.ts"),
        "import { type A } from \"./a\";\nexport type B = { a?: A };\n",
    );
    commit_all(temp.path(), "initial");

    write(
        temp.path().join("src/a.ts"),
        "import { type B } from \"./b\";\nexport type A = { b?: B };\nconst API_KEY = \"abc123xyz987\";\n",
    );
    let first_changed = scan_changed_json(temp.path(), &["--changed", "--profile", "strict"]);
    assert_eq!(
        first_changed["cache_telemetry"]["changed_files"][0]["cache_status"],
        "miss"
    );

    let cache_dir = temp.path().join(".repopilot/cache");
    for name in ["file_hashes.json", "file_roles.json", "findings.json"] {
        force_cache_schema(cache_dir.join(name).as_path(), 4);
    }
    clear_file_roles_deferred_imports(&cache_dir.join("file_roles.json"));
    force_context_graph_resolver(&cache_dir.join("repo_context.json"), "context-graph-v2");
    clear_context_graph_deferred_imports(&cache_dir.join("repo_context.json"));

    let upgraded_changed = scan_changed_json(temp.path(), &["--changed", "--profile", "strict"]);
    assert_eq!(
        upgraded_changed["cache_telemetry"]["changed_files"][0]["cache_status"], "miss",
        "v4 file-role caches must be rejected after inline TS type-only semantics changed: {upgraded_changed:#?}"
    );
    assert_eq!(upgraded_changed["context_graph_cache"]["status"], "miss");
    assert_rule_absent(&upgraded_changed, "architecture.circular-dependency");
}

#[test]
fn changed_scan_rejects_pre_deferred_imports_file_roles_cache() {
    // A v3 file-roles cache predates `FileRoleEntry::deferred_imports`. With
    // `#[serde(default)]` it would rehydrate as `[]`, turn a deferred import back
    // into an eager edge, and resurrect a phantom cycle on the next cache hit.
    // The current cache schema must reject it, forcing re-analysis that restores
    // the deferred imports.
    let temp = tempdir().expect("temp dir");
    init_repo(temp.path());
    write(temp.path().join("app/__init__.py"), "");
    write(temp.path().join("app/models.py"), "thing = object()\n");
    write(
        temp.path().join("app/views.py"),
        "def handler():\n    from app.models import thing\n    return thing\n",
    );
    commit_all(temp.path(), "initial");

    // Establish a deferred cycle: models -> views (eager), views -> models (deferred).
    write(
        temp.path().join("app/models.py"),
        "import app.views\nthing = object()\nAPI_KEY = \"abc123xyz987\"\n",
    );
    let first_changed = scan_changed_json(temp.path(), &["--changed"]);
    assert_eq!(
        first_changed["cache_telemetry"]["changed_files"][0]["cache_status"],
        "miss"
    );

    // Downgrade the persisted file-roles cache to the pre-deferred schema.
    force_cache_schema(
        temp.path()
            .join(".repopilot/cache/file_roles.json")
            .as_path(),
        3,
    );

    let cached_changed = scan_changed_json(temp.path(), &["--changed"]);
    assert_eq!(
        cached_changed["cache_telemetry"]["changed_files"][0]["cache_status"], "miss",
        "a v3 file-roles cache must be rejected, not reused: {cached_changed:#?}"
    );
    let cycles = &cached_changed["context_graph_summary"]["cycles"];
    assert!(
        cycles.as_array().is_none_or(Vec::is_empty),
        "re-analysis must restore deferred imports and not report an eager cycle: {cached_changed:#?}"
    );
}

#[test]
fn changed_scan_invalidates_repo_context_cache_after_branch_switch() {
    let temp = tempdir().expect("temp dir");
    init_repo(temp.path());
    write(temp.path().join("src/old.rs"), "pub fn old() {}\n");
    write(
        temp.path().join("src/lib.rs"),
        "use crate::old;\npub fn live() { old::old(); }\n",
    );
    commit_all(temp.path(), "initial");

    let initial_graph = scan_graph_json(temp.path());
    assert_eq!(initial_graph["context_graph_cache"]["status"], "write");

    git(temp.path(), &["checkout", "-b", "feature"]);
    write(temp.path().join("src/new.rs"), "pub fn new() {}\n");
    write(
        temp.path().join("src/lib.rs"),
        "use crate::new;\npub fn live() { new::new(); }\n",
    );
    commit_all(temp.path(), "feature graph");

    let changed = scan_changed_json(temp.path(), &["--since", "main"]);

    assert_eq!(changed["context_graph_cache"]["status"], "miss");
    assert!(
        top_dependencies_include(&changed, "src/new.rs"),
        "branch-local graph should be rebuilt instead of patched from another branch: {changed:#?}"
    );
}

#[test]
fn since_scan_uses_base_ref_scope() {
    let temp = tempdir().expect("temp dir");
    init_repo(temp.path());
    write(temp.path().join("src/lib.rs"), "pub fn live() {}\n");
    commit_all(temp.path(), "initial");
    git(temp.path(), &["checkout", "-b", "feature"]);
    write(
        temp.path().join("src/lib.rs"),
        "pub fn live() {}\nconst API_KEY: &str = \"abc123xyz987\";\n",
    );
    commit_all(temp.path(), "feature secret");

    let json = scan_changed_json(temp.path(), &["--since", "main"]);

    assert_eq!(json["mode"], "changed");
    assert_eq!(json["base_ref"], "main");
    assert_eq!(json["changed_files_count"], 1);
    assert_rule_present(&json, "security.secret-candidate");
}

#[test]
fn cache_clear_removes_cache_and_is_idempotent() {
    let temp = tempdir().expect("temp dir");
    init_repo(temp.path());
    write(temp.path().join("src/lib.rs"), "pub fn live() {}\n");
    commit_all(temp.path(), "initial");
    write(
        temp.path().join("src/lib.rs"),
        "pub fn live() {}\nconst API_KEY: &str = \"abc123xyz987\";\n",
    );
    scan_changed_json(temp.path(), &["--changed"]);

    let cache_dir = temp.path().join(".repopilot/cache");
    assert!(cache_dir.is_dir());

    clear_cache(temp.path());
    assert!(!cache_dir.exists());
    clear_cache(temp.path());
    assert!(!cache_dir.exists());
}

#[test]
fn changed_scan_rejects_invalid_flag_combinations() {
    let temp = tempdir().expect("temp dir");

    let both = repopilot()
        .args(["scan", ".", "--changed", "--since", "main"])
        .current_dir(temp.path())
        .output()
        .expect("run scan");
    assert_eq!(both.status.code(), Some(2));

    let workspace = repopilot()
        .args(["scan", ".", "--workspace", "--changed"])
        .current_dir(temp.path())
        .output()
        .expect("run scan");
    assert_eq!(workspace.status.code(), Some(2));
}

fn scan_changed_json(root: &Path, args: &[&str]) -> Value {
    let output = repopilot()
        .args(["scan", ".", "--format", "json"])
        .args(args)
        .current_dir(root)
        .output()
        .expect("run repopilot scan");

    assert!(
        output.status.success(),
        "scan failed\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );

    serde_json::from_slice(&output.stdout).expect("json output")
}

// The context graph (and its cache status) is part of the scan report, so a full
// scan is the supported way to observe it now that `inspect graph` is gone.
fn scan_graph_json(root: &Path) -> Value {
    let output = repopilot()
        .args(["scan", ".", "--format", "json"])
        .current_dir(root)
        .output()
        .expect("run repopilot scan");

    assert!(
        output.status.success(),
        "scan failed\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );

    serde_json::from_slice(&output.stdout).expect("json output")
}

fn top_dependencies_include(json: &Value, expected_path: &str) -> bool {
    json["context_graph_summary"]["top_dependencies"]
        .as_array()
        .into_iter()
        .flatten()
        .any(|metric| metric["path"] == expected_path)
}

fn clear_cache(root: &Path) {
    let output = repopilot()
        .args(["cache", "clear", "."])
        .current_dir(root)
        .output()
        .expect("run cache clear");

    assert!(
        output.status.success(),
        "cache clear failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );
}

fn rewrite_cached_finding_title(path: &Path, title: &str) {
    let mut value = read_json(path);
    value["entries"][0]["findings"][0]["title"] = Value::String(title.to_string());
    fs::write(
        path,
        serde_json::to_string_pretty(&value).expect("render findings cache"),
    )
    .expect("write findings cache");
}

fn force_cache_schema(path: &Path, schema_version: u64) {
    let mut value = read_json(path);
    value["schema_version"] = Value::Number(schema_version.into());
    write_json(path, &value);
}

fn clear_file_roles_deferred_imports(path: &Path) {
    let mut value = read_json(path);
    for entry in value["entries"].as_array_mut().expect("file role entries") {
        entry["deferred_imports"] = Value::Array(Vec::new());
    }
    write_json(path, &value);
}

fn remove_cached_role(path: &Path, role: &str) {
    let mut value = read_json(path);
    for entry in value["entries"].as_array_mut().expect("file role entries") {
        if let Some(roles) = entry["roles"].as_array_mut() {
            roles.retain(|value| value.as_str() != Some(role));
        }
    }
    write_json(path, &value);
}

fn set_cached_rule_severity(path: &Path, rule_id: &str, severity: &str) {
    let mut value = read_json(path);
    for entry in value["entries"].as_array_mut().expect("finding entries") {
        for finding in entry["findings"].as_array_mut().expect("findings") {
            if finding["rule_id"] == rule_id {
                finding["severity"] = Value::String(severity.to_string());
            }
        }
    }
    write_json(path, &value);
}

fn force_context_graph_resolver(path: &Path, resolver_version: &str) {
    let mut value = read_json(path);
    value["resolver_version"] = Value::String(resolver_version.to_string());
    write_json(path, &value);
}

fn clear_context_graph_deferred_imports(path: &Path) {
    let mut value = read_json(path);
    value["graph"]["deferred_edges"] = serde_json::json!({});
    for node in value["graph"]["nodes"]
        .as_array_mut()
        .expect("context nodes")
    {
        node["deferred_imports"] = Value::Array(Vec::new());
    }
    write_json(path, &value);
}

fn write_json(path: &Path, value: &Value) {
    fs::write(
        path,
        serde_json::to_string_pretty(value).expect("render cache"),
    )
    .expect("write cache");
}

fn read_json(path: &Path) -> Value {
    serde_json::from_slice(&fs::read(path).expect("read json")).expect("json")
}

fn first_finding_title(json: &Value) -> &str {
    json["findings"][0]["title"]
        .as_str()
        .expect("finding title")
}

fn secret_candidate_count(json: &Value) -> usize {
    json["findings"]
        .as_array()
        .into_iter()
        .flatten()
        .filter(|finding| finding["rule_id"] == "security.secret-candidate")
        .count()
}

fn assert_rule_present(json: &Value, rule_id: &str) {
    assert!(
        json["findings"]
            .as_array()
            .into_iter()
            .flatten()
            .any(|finding| finding["rule_id"] == rule_id),
        "expected {rule_id} in findings: {json:#?}"
    );
}

fn finding_for_rule<'a>(json: &'a Value, rule_id: &str) -> &'a Value {
    json["findings"]
        .as_array()
        .into_iter()
        .flatten()
        .find(|finding| finding["rule_id"] == rule_id)
        .unwrap_or_else(|| panic!("expected {rule_id} in findings: {json:#?}"))
}

fn assert_rule_absent(json: &Value, rule_id: &str) {
    assert!(
        json["findings"]
            .as_array()
            .into_iter()
            .flatten()
            .all(|finding| finding["rule_id"] != rule_id),
        "did not expect {rule_id} in findings: {json:#?}"
    );
}

fn assert_changed_reason(json: &Value, reason: &str, count: u64) {
    assert!(
        json["cache_telemetry"]["changed_file_reasons"]
            .as_array()
            .into_iter()
            .flatten()
            .any(|item| item["reason"] == reason && item["count"] == count),
        "expected changed file reason {reason} ({count}) in {json:#?}"
    );
}

fn init_repo(root: &Path) {
    git(root, &["init"]);
    git(root, &["checkout", "-B", "main"]);
    git(root, &["config", "user.email", "repopilot@example.com"]);
    git(root, &["config", "user.name", "RepoPilot Test"]);
}

fn commit_all(root: &Path, message: &str) {
    git(root, &["add", "."]);
    git(root, &["commit", "-m", message]);
}

fn git(root: &Path, args: &[&str]) {
    let output = Command::new("git")
        .args(args)
        .current_dir(root)
        .output()
        .expect("run git");
    assert!(
        output.status.success(),
        "git {:?} failed\nstdout:\n{}\nstderr:\n{}",
        args,
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
}

fn write(path: std::path::PathBuf, content: &str) {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).expect("create parent");
    }
    fs::write(path, content).expect("write file");
}
