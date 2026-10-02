//! Agent integrations beyond Claude Code and Cursor: the Codex plugin manifest,
//! the Gemini CLI settings, the Copilot setup workflow, and the AGENTS.md
//! snippet. The session scripts are shared, so the Codex and Gemini hook
//! inputs are run through them here as those agents send them.
#![cfg(unix)]

use serde_json::Value;
use std::fs;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::{Command, Output, Stdio};
use tempfile::tempdir;

const PLUGIN: &str = "integrations/claude-code/repopilot";

fn root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

fn json(path: &Path) -> Value {
    serde_json::from_str(&fs::read_to_string(path).expect("read json")).expect("valid json")
}

#[test]
fn codex_plugin_manifest_points_at_shipped_files() {
    let plugin = root().join(PLUGIN);
    let manifest = json(&plugin.join(".codex-plugin/plugin.json"));
    let claude = json(&plugin.join(".claude-plugin/plugin.json"));
    assert_eq!(manifest["name"], "repopilot");
    assert_eq!(
        manifest["version"], claude["version"],
        "one plugin version for both agents"
    );
    for field in ["skills", "mcpServers", "hooks"] {
        let relative = manifest[field].as_str().expect(field);
        assert!(plugin.join(relative).exists(), "{field}: {relative}");
    }
    let hooks = json(&plugin.join(manifest["hooks"].as_str().unwrap()));
    for event in ["SessionStart", "Stop"] {
        let command = hooks["hooks"][event][0]["hooks"][0]["command"]
            .as_str()
            .expect("command");
        let script = command
            .split("$PLUGIN_ROOT/")
            .nth(1)
            .and_then(|rest| rest.split('"').next())
            .expect("script under $PLUGIN_ROOT");
        assert!(plugin.join(script).is_file(), "{event}: {script}");
    }
}

#[test]
fn gemini_settings_run_the_shared_scripts_and_register_mcp() {
    let settings = json(&root().join("integrations/gemini/settings.json"));
    let start = settings["hooks"]["SessionStart"][0]["hooks"][0]["command"]
        .as_str()
        .unwrap();
    let stop = settings["hooks"]["AfterAgent"][0]["hooks"][0]["command"]
        .as_str()
        .unwrap();
    assert_eq!(start, "sh .gemini/hooks/repopilot-snapshot.sh");
    assert_eq!(stop, "sh .gemini/hooks/repopilot-guard.sh");
    assert_eq!(settings["mcpServers"]["repopilot"]["args"][0], "mcp");
}

#[test]
fn copilot_setup_installs_repopilot_in_the_required_job() {
    let workflow = fs::read_to_string(root().join("integrations/copilot/copilot-setup-steps.yml"))
        .expect("workflow");
    let parsed: serde_yaml::Value = serde_yaml::from_str(&workflow).expect("yaml");
    let steps = parsed["jobs"]["copilot-setup-steps"]["steps"]
        .as_sequence()
        .expect("copilot-setup-steps job");
    let runs: Vec<&str> = steps
        .iter()
        .filter_map(|step| step["run"].as_str())
        .collect();
    assert!(runs.contains(&"npm install -g repopilot"), "{runs:?}");
    assert!(runs.contains(&"repopilot snapshot"), "{runs:?}");
}

#[test]
fn agents_snippet_names_the_console_block_it_asks_to_act_on() {
    let snippet =
        fs::read_to_string(root().join("integrations/agents/AGENTS.md")).expect("snippet");
    let console = fs::read_to_string(root().join("src/review/render/console/weakened.rs"))
        .expect("console block source");
    assert!(console.contains("Checks this change weakened"));
    assert!(snippet.contains("Checks this change weakened"));
    assert!(snippet.contains("repopilot snapshot"));
    assert!(snippet.contains("repopilot review --since-snapshot"));
}

const BEFORE: &str = r#"describe("cart", () => {
  it("sums line items", () => {
    expect(total([2, 3])).toBe(5);
  });
  it("applies the discount", () => {
    expect(total([10], 0.1)).toBe(9);
  });
});
"#;

fn hook(root: &Path, script: &str, stdin: &str) -> Output {
    let bin = Path::new(env!("CARGO_BIN_EXE_repopilot"))
        .parent()
        .unwrap()
        .to_path_buf();
    let path = format!(
        "{}:{}",
        bin.display(),
        std::env::var("PATH").unwrap_or_default()
    );
    let mut child = Command::new("sh")
        .arg(self::root().join(PLUGIN).join("scripts").join(script))
        .current_dir(root)
        .env("PATH", path)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("hook starts");
    child
        .stdin
        .take()
        .unwrap()
        .write_all(stdin.as_bytes())
        .unwrap();
    child.wait_with_output().expect("hook finishes")
}

fn git(root: &Path, args: &[&str]) {
    assert!(
        Command::new("git")
            .args(args)
            .current_dir(root)
            .status()
            .unwrap()
            .success()
    );
}

#[test]
fn codex_and_gemini_stop_inputs_block_a_weakened_session_once() {
    for (start, stop, again) in [
        (
            r#"{"hook_event_name":"SessionStart","source":"startup","session_id":"s","cwd":"."}"#,
            r#"{"hook_event_name":"Stop","turn_id":"t1","stop_hook_active":false,"last_assistant_message":"Done.","session_id":"s","cwd":"."}"#,
            r#"{"hook_event_name":"Stop","turn_id":"t2","stop_hook_active":true,"last_assistant_message":"Done.","session_id":"s","cwd":"."}"#,
        ),
        (
            r#"{"hook_event_name":"SessionStart","source":"startup","session_id":"s","cwd":".","timestamp":"2026-10-02T00:00:00Z"}"#,
            r#"{"hook_event_name":"AfterAgent","prompt":"fix the cart","prompt_response":"Done.","stop_hook_active":false,"session_id":"s","cwd":"."}"#,
            r#"{"hook_event_name":"AfterAgent","prompt":"fix the cart","prompt_response":"Done.","stop_hook_active":true,"session_id":"s","cwd":"."}"#,
        ),
    ] {
        let temp = tempdir().expect("temp repo");
        let repo = temp.path();
        git(repo, &["init", "-q"]);
        git(repo, &["config", "user.email", "repopilot@example.invalid"]);
        git(repo, &["config", "user.name", "RepoPilot Test"]);
        fs::create_dir_all(repo.join("src")).unwrap();
        fs::write(repo.join("src/cart.test.ts"), BEFORE).unwrap();
        git(repo, &["add", "."]);
        git(repo, &["commit", "-qm", "before"]);

        assert!(hook(repo, "snapshot.sh", start).status.success());
        let weakened = BEFORE.replace("it(\"applies", "it.skip(\"applies");
        fs::write(repo.join("src/cart.test.ts"), weakened).unwrap();

        let blocked = hook(repo, "guard.sh", stop);
        let feedback = String::from_utf8_lossy(&blocked.stderr);
        assert_eq!(blocked.status.code(), Some(2), "{feedback}");
        assert!(
            feedback.contains("test skipped — src/cart.test.ts"),
            "{feedback}"
        );
        assert_eq!(hook(repo, "guard.sh", again).status.code(), Some(0));
    }
}

#[test]
fn gemini_extension_runs_the_shared_scripts_from_the_repository_root() {
    let manifest = json(&root().join("gemini-extension.json"));
    let plugin = json(&root().join(PLUGIN).join(".claude-plugin/plugin.json"));
    assert_eq!(manifest["name"], "repopilot");
    assert_eq!(
        manifest["version"], plugin["version"],
        "one version for every agent"
    );
    assert_eq!(
        manifest["mcpServers"]["repopilot"]["args"],
        serde_json::json!(["mcp", "--root", "${workspacePath}"])
    );
    let hooks = json(&root().join("hooks/hooks.json"));
    for (event, script) in [("SessionStart", "snapshot.sh"), ("AfterAgent", "guard.sh")] {
        let command = hooks["hooks"][event][0]["hooks"][0]["command"]
            .as_str()
            .expect("command");
        let relative = command
            .split("${extensionPath}/")
            .nth(1)
            .and_then(|rest| rest.split('"').next())
            .expect("script under ${extensionPath}");
        assert!(relative.ends_with(script), "{event}: {relative}");
        assert!(root().join(relative).is_file(), "{event}: {relative}");
    }
}

#[test]
fn mcp_registry_entry_is_owned_by_the_published_packages() {
    let server = json(&root().join("server.json"));
    let package = json(&root().join("package.json"));
    let name = server["name"].as_str().expect("name");
    assert_eq!(package["mcpName"], name, "npm ownership check");
    let readme = fs::read_to_string(root().join("README.md")).expect("README");
    assert!(
        readme.contains(&format!("mcp-name: {name}")),
        "crates.io ownership check"
    );
    assert!(server["description"].as_str().unwrap().chars().count() <= 100);
    for entry in server["packages"].as_array().expect("packages") {
        assert_eq!(entry["identifier"], "repopilot");
        assert_eq!(entry["version"], server["version"]);
        let args: Vec<&str> = entry["packageArguments"]
            .as_array()
            .unwrap()
            .iter()
            .map(|arg| arg["value"].as_str().unwrap())
            .collect();
        assert_eq!(args, ["mcp", "."]);
    }
}
