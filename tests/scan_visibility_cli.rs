use serde_json::Value;
use std::fs;
use std::process::Command;
use tempfile::tempdir;

fn repopilot() -> Command {
    Command::new(env!("CARGO_BIN_EXE_repopilot"))
}

#[test]
fn default_scan_hides_source_without_test_but_strict_includes_it() {
    let temp = tempdir().expect("temp dir");
    let root = temp.path();
    write(
        root.join("src/payment.rs"),
        "pub fn charge() -> bool { true }\n",
    );

    let default = scan_json(root, &[]);
    assert_rule_absent(&default, "testing.source-without-test");
    assert!(
        default["hidden_suggestions_count"].as_u64().unwrap_or(0) >= 1,
        "default report should count hidden testing suggestions: {default:#?}"
    );

    let strict = scan_json(root, &["--profile", "strict"]);
    assert_rule_present(&strict, "testing.source-without-test");
    assert_rule_present(&strict, "testing.missing-test-folder");
}

#[test]
fn default_scan_hides_script_process_exit_but_reports_library_process_exit() {
    let temp = tempdir().expect("temp dir");
    let root = temp.path();
    write(root.join("scripts/check.js"), "process.exit(1);\n");
    write(
        root.join("src/lib/runtime.js"),
        "export function stop() { process.exit(1); }\n",
    );

    let json = scan_json(root, &[]);
    let runtime_findings =
        findings_for_rule(&json, "language.javascript.runtime-exit-risk").collect::<Vec<_>>();

    assert_eq!(
        runtime_findings.len(),
        1,
        "default report should hide script process.exit and keep library process.exit: {json:#?}"
    );
    assert!(
        first_path(runtime_findings[0]).ends_with("src/lib/runtime.js"),
        "reported process.exit should be in reusable library code"
    );

    let strict = scan_json(root, &["--profile", "strict"]);
    assert_eq!(
        findings_for_rule(&strict, "language.javascript.runtime-exit-risk").count(),
        2,
        "strict report should include script and library process.exit findings: {strict:#?}"
    );
}

#[test]
fn default_scan_hides_cli_package_process_exit_but_reports_non_cli_package() {
    // A monorepo with two packages that both call `process.exit` from a
    // `commands/` directory. Only the package that declares `package.json#bin`
    // is a CLI tool, so its exit is downgraded to Low (hidden by default, kept in
    // strict). The non-CLI package's `commands/` is a CQRS/domain command, so its
    // host-terminating exit stays a default-visible High — the false-negative
    // guard for the old path-only `commands/` heuristic.
    let temp = tempdir().expect("temp dir");
    let root = temp.path();
    write(
        root.join("package.json"),
        r#"{ "name": "monorepo", "workspaces": ["packages/*"] }"#,
    );
    write(
        root.join("packages/cli/package.json"),
        r#"{ "name": "@app/cli", "bin": { "app": "./bin/app.js" } }"#,
    );
    write(
        root.join("packages/cli/src/commands/run.ts"),
        "export const run = () => { process.exit(1); };\n",
    );
    // A reusable module inside the CLI package: NOT a command handler, so its exit
    // stays default-visible High even though the package declares `bin`.
    write(
        root.join("packages/cli/src/lib/parser.ts"),
        "export function parse(s: string) { if (!s) { process.exit(1); } }\n",
    );
    write(
        root.join("packages/web/package.json"),
        r#"{ "name": "@app/web" }"#,
    );
    write(
        root.join("packages/web/src/domain/commands/handler.ts"),
        "export function handle() { process.exit(1); }\n",
    );

    let json = scan_json(root, &[]);
    let default_paths = findings_for_rule(&json, "language.javascript.runtime-exit-risk")
        .map(first_path)
        .collect::<Vec<_>>();
    assert_eq!(
        default_paths.len(),
        2,
        "default should hide only the CLI command handler's exit: {json:#?}"
    );
    assert!(
        default_paths.iter().any(|p| p.ends_with("handler.ts")),
        "the non-CLI (domain) command exit must stay visible"
    );
    assert!(
        default_paths.iter().any(|p| p.ends_with("parser.ts")),
        "a reusable module's exit inside the CLI package must stay visible"
    );
    assert!(
        !default_paths.iter().any(|p| p.ends_with("run.ts")),
        "the CLI command handler's exit must be hidden by default"
    );

    let strict = scan_json(root, &["--profile", "strict"]);
    assert_eq!(
        findings_for_rule(&strict, "language.javascript.runtime-exit-risk").count(),
        3,
        "strict report should include all three process.exit findings: {strict:#?}"
    );
}

#[test]
fn default_scan_reports_unwrap_on_external_parse_path() {
    let temp = tempdir().expect("temp dir");
    let root = temp.path();
    write(
        root.join("src/parser.rs"),
        "pub fn parse_port(raw: &str) -> u16 { raw.parse::<u16>().unwrap() }\n",
    );

    let json = scan_json(root, &[]);
    let rust_findings = findings_for_rule(&json, "language.rust.panic-risk").collect::<Vec<_>>();

    assert_eq!(
        rust_findings.len(),
        1,
        "external parse unwrap should remain visible in default report: {json:#?}"
    );
    assert_eq!(rust_findings[0]["severity"], "HIGH");
}

#[test]
fn default_scan_hides_mutex_lock_unwrap_named_db() {
    // A `Mutex` field named `db` is common; `self.db.lock().unwrap()` is a
    // poisoned-lock panic, not an external database failure, so it must not
    // surface as a visible HIGH finding the way an external parse unwrap does.
    let temp = tempdir().expect("temp dir");
    let root = temp.path();
    write(
        root.join("src/state.rs"),
        "pub struct App { db: std::sync::Mutex<u32> }\n\
         impl App { pub fn bump(&self) { let mut g = self.db.lock().unwrap(); *g += 1; } }\n",
    );

    let json = scan_json(root, &[]);

    assert_rule_absent(&json, "language.rust.panic-risk");
}

#[test]
fn default_scan_hides_serde_serialization_unwrap() {
    // `serde_json::to_string(&value).unwrap()` serializes an owned value to
    // JSON — effectively infallible and in-process. It must not surface as a
    // visible HIGH the way a genuine `serde_json::from_str` deserialization does.
    let temp = tempdir().expect("temp dir");
    let root = temp.path();
    write(
        root.join("src/store.rs"),
        "pub fn dump(v: &Vec<u32>) -> String { serde_json::to_string(v).unwrap() }\n",
    );

    let json = scan_json(root, &[]);

    assert_rule_absent(&json, "language.rust.panic-risk");
}

#[test]
fn default_scan_keeps_real_secret_and_private_key_candidates() {
    let temp = tempdir().expect("temp dir");
    let root = temp.path();
    write(
        root.join("src/config.rs"),
        "const API_KEY: &str = \"xK9mQ2pL8rT5vN3wY7\";\n",
    );
    write(
        root.join("src/key.pem"),
        "-----BEGIN RSA PRIVATE KEY-----\nvery-secret-key-bytes\n-----END RSA PRIVATE KEY-----\n",
    );

    let json = scan_json(root, &[]);
    let secret_findings = findings_for_rule(&json, "security.secret-candidate").collect::<Vec<_>>();

    assert_eq!(
        secret_findings.len(),
        1,
        "mixed-case/digit high-entropy credential must remain default-visible: {json:#?}"
    );
    assert_eq!(secret_findings[0]["confidence"], "HIGH");
    assert_rule_present(&json, "security.private-key-candidate");
}

#[test]
fn default_scan_hides_lowercase_slug_secrets_but_strict_keeps_them() {
    let temp = tempdir().expect("temp dir");
    let root = temp.path();
    write(
        root.join("src/config.ts"),
        "export const CONFIG = {\n\
           OAI_API_KEY: \"excalidraw-oai-api-key\",\n\
           password: \"correct-horse-battery-staple\",\n\
           client_secret: \"prod-service-access-token\",\n\
           auth_token: \"internal-service-access-token\",\n\
         };\n",
    );

    let default = scan_json(root, &[]);
    assert_rule_absent(&default, "security.secret-candidate");

    let strict = scan_json(root, &["--profile", "strict"]);
    let secret_findings =
        findings_for_rule(&strict, "security.secret-candidate").collect::<Vec<_>>();

    assert_eq!(
        secret_findings.len(),
        4,
        "strict profile must retain lowercase passphrase/token-like slug findings: {strict:#?}"
    );
    assert!(
        secret_findings
            .iter()
            .all(|finding| finding["confidence"] == "LOW"),
        "slug findings should be low-confidence strict-only suggestions: {strict:#?}"
    );
}

#[test]
fn strict_scan_keeps_distinct_secret_occurrences_with_colliding_baseline_keys() {
    let temp = tempdir().expect("temp dir");
    let root = temp.path();
    write(
        root.join("src/config.py"),
        "API_KEY = \"abc123xyz987\"\nAPI_KEY = \"def456uvw654\"\n",
    );

    let json = scan_json(root, &["--profile", "strict"]);
    let secret_findings = findings_for_rule(&json, "security.secret-candidate").collect::<Vec<_>>();

    assert_eq!(
        secret_findings.len(),
        2,
        "distinct hardcoded secret occurrences must not be merged just because their baseline keys normalize to the same id: {json:#?}"
    );
    assert_eq!(secret_findings[0]["id"], secret_findings[1]["id"]);
    assert_ne!(
        secret_findings[0]["evidence"][0]["line_start"],
        secret_findings[1]["evidence"][0]["line_start"]
    );
}

#[test]
fn default_scan_hides_large_file_and_long_function_threshold_findings() {
    let temp = tempdir().expect("temp dir");
    let root = temp.path();
    write(
        root.join("repopilot.toml"),
        "[architecture]\nmax_file_lines = 3\nhuge_file_lines = 20\nmax_function_lines = 2\n",
    );
    write(
        root.join("src/lib.rs"),
        "pub fn large() {\nlet a = 1;\nlet b = 2;\nlet c = 3;\nprintln!(\"{}{}{}\", a, b, c);\n}\n",
    );

    let default = scan_json(root, &[]);
    assert_rule_absent(&default, "architecture.large-file");
    assert_rule_absent(&default, "code-quality.long-function");
    assert!(
        default["hidden_suggestions_count"].as_u64().unwrap_or(0) >= 2,
        "default report should count hidden maintainability suggestions: {default:#?}"
    );

    let strict = scan_json(root, &["--profile", "strict"]);
    assert_rule_present(&strict, "architecture.large-file");
    assert_rule_present(&strict, "code-quality.long-function");
}

#[test]
fn default_scan_surfaces_stable_import_graph_risks_and_reports_raw_metrics() {
    let temp = tempdir().expect("temp dir");
    let root = temp.path();
    write(
        root.join("repopilot.toml"),
        "[architecture]\nmax_fan_out = 2\ninstability_hub_min_fan_in = 999\n",
    );
    write(
        root.join("src/app.ts"),
        r#"
        import { b } from "./b";
        import { c } from "./c";
        import { d } from "./d";

        export function app() { return b() + c() + d(); }
        "#,
    );
    write(root.join("src/b.ts"), "export function b() { return 1; }\n");
    write(root.join("src/c.ts"), "export function c() { return 1; }\n");
    write(root.join("src/d.ts"), "export function d() { return 1; }\n");
    write(root.join("src/leaf_test.ts"), "test('leaf', () => {});\n");

    let default = scan_json(root, &[]);

    assert_rule_present(&default, "architecture.excessive-fan-out");
    assert!(
        default["raw_findings_count"].as_u64().unwrap_or(0)
            >= default["visible_findings_count"].as_u64().unwrap_or(0),
        "raw count should preserve findings before default visibility filtering: {default:#?}"
    );
    assert_eq!(
        default["signal_quality"], default["visible_signal_quality"],
        "legacy signal_quality should remain the visible/report quality alias"
    );
    assert!(
        default["raw_signal_quality"]["findings_total"]
            .as_u64()
            .unwrap_or(0)
            >= default["visible_signal_quality"]["findings_total"]
                .as_u64()
                .unwrap_or(0),
        "raw quality should include hidden strict-only suggestions: {default:#?}"
    );
}

#[test]
fn verbose_does_not_change_default_visibility() {
    let temp = tempdir().expect("temp dir");
    let root = temp.path();
    write(
        root.join("src/payment.rs"),
        "pub fn charge() -> bool { true }\n",
    );

    let default = scan_json(root, &[]);
    let verbose = scan_json(root, &["--verbose"]);

    assert_eq!(rule_ids(&default), rule_ids(&verbose));
    assert_eq!(
        default["hidden_suggestions_count"],
        verbose["hidden_suggestions_count"]
    );
}

fn scan_json(root: &std::path::Path, extra_args: &[&str]) -> Value {
    let output = repopilot()
        .args(["scan", ".", "--format", "json"])
        .args(extra_args)
        .current_dir(root)
        .output()
        .expect("run repopilot scan");

    assert!(
        output.status.success(),
        "scan failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );

    serde_json::from_slice(&output.stdout).expect("json output")
}

fn assert_rule_present(json: &Value, rule_id: &str) {
    assert!(
        findings_for_rule(json, rule_id).next().is_some(),
        "expected {rule_id} in findings: {json:#?}"
    );
}

fn assert_rule_absent(json: &Value, rule_id: &str) {
    assert!(
        findings_for_rule(json, rule_id).next().is_none(),
        "did not expect {rule_id} in findings: {json:#?}"
    );
}

fn findings_for_rule<'a>(
    json: &'a Value,
    rule_id: &'a str,
) -> impl Iterator<Item = &'a Value> + 'a {
    json["findings"]
        .as_array()
        .into_iter()
        .flatten()
        .filter(move |finding| finding["rule_id"] == rule_id)
}

fn first_path(finding: &Value) -> &str {
    finding["evidence"][0]["path"]
        .as_str()
        .expect("finding path")
}

fn rule_ids(json: &Value) -> Vec<String> {
    let mut ids = json["findings"]
        .as_array()
        .into_iter()
        .flatten()
        .filter_map(|finding| finding["rule_id"].as_str().map(str::to_string))
        .collect::<Vec<_>>();
    ids.sort();
    ids
}

fn write(path: std::path::PathBuf, content: &str) {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).expect("create parent");
    }
    fs::write(path, content).expect("write file");
}
