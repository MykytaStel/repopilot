use std::env;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, Output};

struct Harness {
    _temp: tempfile::TempDir,
    root: PathBuf,
    bin_dir: PathBuf,
    cargo_log: PathBuf,
    scan_log: PathBuf,
    stale_log: PathBuf,
    current: PathBuf,
    explicit: PathBuf,
}

impl Harness {
    fn new(current_relative: &str) -> Self {
        let temp = tempfile::tempdir().expect("temp dir");
        let root = temp.path().join("repo");
        let bin_dir = root.join("fake-bin");
        let cargo_log = root.join("cargo.log");
        let scan_log = root.join("scan.log");
        let stale_log = root.join("stale.log");
        let current = root.join(current_relative);
        let explicit = root.join("external/repopilot");
        fs::create_dir_all(root.join("scripts")).expect("scripts dir");
        fs::create_dir_all(root.join("tests/zoo/snapshots")).expect("snapshot dir");
        fs::create_dir_all(root.join(".zoo/sample")).expect("zoo clone");
        fs::create_dir_all(&bin_dir).expect("bin dir");
        copy_script(&root, "zoo.py");
        copy_script(&root, "zoo_scanner.py");
        fs::write(root.join("Cargo.toml"), CARGO_TOML).expect("Cargo.toml");
        fs::write(root.join("tests/zoo/manifest.toml"), MANIFEST).expect("manifest");
        fs::write(root.join("tests/zoo/snapshots/sample.json"), SNAPSHOT).expect("snapshot");
        write_executable(&bin_dir.join("cargo"), fake_cargo());
        write_executable(&bin_dir.join("git"), fake_git());
        write_executable(&current, fake_repopilot());
        write_executable(&explicit, fake_repopilot());
        write_executable(&root.join("target/release/repopilot"), stale_repopilot());
        Self {
            _temp: temp,
            root,
            bin_dir,
            cargo_log,
            scan_log,
            stale_log,
            current,
            explicit,
        }
    }

    fn run(&self, args: &[&str], extra_env: &[(&str, String)]) -> Output {
        let mut cmd = Command::new("python3");
        cmd.arg("scripts/zoo.py").args(args).current_dir(&self.root);
        cmd.env("PATH", path_with_fake_bin(&self.bin_dir));
        cmd.env("FAKE_CARGO_LOG", &self.cargo_log);
        cmd.env("FAKE_SCAN_LOG", &self.scan_log);
        cmd.env("FAKE_STALE_LOG", &self.stale_log);
        cmd.env("FAKE_CARGO_EXECUTABLE", &self.current);
        for (key, value) in extra_env {
            cmd.env(key, value);
        }
        cmd.output().expect("run zoo.py")
    }
}

#[test]
fn workspace_mode_builds_once_ignores_stale_target_binary_and_keeps_snapshots() {
    let h = Harness::new("fresh/repopilot");
    let output = h.run(&["scan", "--only", "sample"], &[]);
    assert_success(&output);
    let stdout = text(&output.stdout);
    assert_contains(&stdout, "scanner mode: workspace");
    assert_contains(&stdout, "scanner profile: release");
    assert_contains(&stdout, format!("scanner binary: {}", h.current.display()));
    assert_contains(&stdout, "scanner version: 0.18.0");
    assert_contains(
        &stdout,
        "workspace commit: 0123456789abcdef0123456789abcdef01234567",
    );
    assert_contains(&stdout, "workspace dirty: false");
    assert!(
        stdout.find("scanner mode: workspace")
            < stdout.find("  scanning sample").or(Some(usize::MAX))
    );
    assert_contains(&stdout, "ok (default-visible: 0)");
    assert_eq!(line_count(&h.cargo_log), 1, "cargo build should run once");
    assert!(
        !h.stale_log.exists(),
        "stale target/release binary was selected"
    );
}

#[test]
fn explicit_repopilot_path_is_used_without_cargo_build() {
    let h = Harness::new("fresh/repopilot");
    let explicit = h.explicit.to_str().unwrap();
    let output = h.run(&["scan", "--only", "sample", "--repopilot", explicit], &[]);
    assert_success(&output);
    let stdout = text(&output.stdout);
    assert_contains(&stdout, "scanner mode: explicit");
    assert_contains(&stdout, format!("scanner binary: {}", h.explicit.display()));
    assert!(
        !h.cargo_log.exists(),
        "explicit mode should not invoke cargo"
    );
    assert!(
        fs::read_to_string(&h.scan_log)
            .unwrap()
            .contains(&h.explicit.display().to_string())
    );
}

#[test]
fn explicit_version_mismatch_requires_opt_in() {
    let h = Harness::new("fresh/repopilot");
    let mismatch = [("FAKE_REPOPILOT_VERSION", "9.9.9".to_string())];
    let explicit = h.explicit.to_str().unwrap();
    let output = h.run(
        &["scan", "--only", "sample", "--repopilot", explicit],
        &mismatch,
    );
    assert_eq!(output.status.code(), Some(3), "{}", text(&output.stderr));
    let stderr = text(&output.stderr);
    assert!(stderr.contains("SCANNER PROVENANCE FAILED"), "{stderr}");
    assert!(stderr.contains("--allow-version-mismatch"), "{stderr}");

    let allowed = h.run(
        &[
            "scan",
            "--only",
            "sample",
            "--repopilot",
            explicit,
            "--allow-version-mismatch",
        ],
        &mismatch,
    );
    assert_success(&allowed);
    assert!(text(&allowed.stdout).contains("workspace version: 0.18.0 (mismatch allowed)"));
}

#[test]
fn workspace_build_uses_cargo_reported_target_dir_and_exe_artifact() {
    let h = Harness::new("custom-target/release/repopilot.exe");
    let target_dir = h.root.join("custom-target");
    let output = h.run(
        &["scan", "--only", "sample"],
        &[("CARGO_TARGET_DIR", target_dir.display().to_string())],
    );
    assert_success(&output);
    let stdout = text(&output.stdout);
    assert_contains(&stdout, "custom-target/release/repopilot.exe");
    let cargo_log = fs::read_to_string(&h.cargo_log).expect("cargo log");
    assert!(cargo_log.contains(&format!("CARGO_TARGET_DIR={}", target_dir.display())));
}

#[test]
fn provenance_rejects_inconsistent_default_and_strict_report_metadata() {
    let h = Harness::new("fresh/repopilot");
    let output = h.run(
        &["scan", "--only", "sample"],
        &[("FAKE_STRICT_SCHEMA_VERSION", "9.99".to_string())],
    );
    assert_eq!(output.status.code(), Some(3), "{}", text(&output.stderr));
    let stderr = text(&output.stderr);
    assert!(stderr.contains("SCANNER PROVENANCE FAILED"), "{stderr}");
    assert!(stderr.contains("different schema versions"), "{stderr}");
    assert!(!text(&output.stdout).contains("  scanning sample"));
}

fn repo_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).to_path_buf()
}

fn copy_script(root: &Path, name: &str) {
    fs::copy(
        repo_root().join("scripts").join(name),
        root.join("scripts").join(name),
    )
    .unwrap_or_else(|err| panic!("copy {name}: {err}"));
}

fn write_executable(path: &Path, contents: &str) {
    fs::create_dir_all(path.parent().expect("parent")).expect("executable parent");
    fs::write(path, contents).expect("write executable");
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        fs::set_permissions(path, fs::Permissions::from_mode(0o755)).expect("chmod");
    }
}

fn path_with_fake_bin(bin_dir: &Path) -> std::ffi::OsString {
    let old_path = env::var_os("PATH").unwrap_or_default();
    let paths = std::iter::once(bin_dir.to_path_buf()).chain(env::split_paths(&old_path));
    env::join_paths(paths).expect("join PATH")
}

fn assert_success(output: &Output) {
    assert!(
        output.status.success(),
        "stdout:\n{}\nstderr:\n{}",
        text(&output.stdout),
        text(&output.stderr)
    );
}

fn assert_contains(haystack: &str, needle: impl AsRef<str>) {
    assert!(haystack.contains(needle.as_ref()), "{haystack}");
}

fn text(bytes: &[u8]) -> String {
    String::from_utf8_lossy(bytes).into_owned()
}

fn line_count(path: &Path) -> usize {
    fs::read_to_string(path).expect("log").lines().count()
}

const CARGO_TOML: &str =
    "[package]\nname = \"repopilot\"\nversion = \"0.18.0\"\nedition = \"2024\"\n";
const MANIFEST: &str = "[[repo]]\nname = \"sample\"\nurl = \"https://example.invalid/sample.git\"\nsha = \"abc123\"\nlanguage = \"rust\"\n";
const SNAPSHOT: &str = "{\n  \"default\": {\n    \"by_priority\": {},\n    \"by_rule\": {},\n    \"fingerprints\": [],\n    \"visible_total\": 0\n  },\n  \"framework\": \"\",\n  \"language\": \"rust\",\n  \"repo\": \"sample\",\n  \"sha\": \"abc123\",\n  \"strict\": {\n    \"by_rule\": {},\n    \"visible_total\": 0\n  }\n}\n";

fn fake_cargo() -> &'static str {
    r#"#!/usr/bin/env python3
import json, os, sys
with open(os.environ["FAKE_CARGO_LOG"], "a", encoding="utf-8") as log:
    log.write(" ".join(sys.argv[1:]) + "|CARGO_TARGET_DIR=" + os.environ.get("CARGO_TARGET_DIR", "") + "\n")
if len(sys.argv) > 1 and sys.argv[1] == "build":
    print(json.dumps({
        "reason": "compiler-artifact",
        "target": {"kind": ["bin"], "name": "repopilot"},
        "executable": os.environ["FAKE_CARGO_EXECUTABLE"],
    }))
    print(json.dumps({"reason": "build-finished", "success": True}))
    raise SystemExit(0)
print("unexpected cargo args: " + repr(sys.argv), file=sys.stderr)
raise SystemExit(9)
"#
}

fn fake_git() -> &'static str {
    r#"#!/usr/bin/env python3
import os, sys
if sys.argv[1:] == ["rev-parse", "HEAD"]:
    print("0123456789abcdef0123456789abcdef01234567")
    raise SystemExit(0)
if sys.argv[1:] == ["status", "--porcelain"]:
    print(os.environ.get("FAKE_GIT_STATUS", ""), end="")
    raise SystemExit(0)
print("unexpected git args: " + repr(sys.argv), file=sys.stderr)
raise SystemExit(9)
"#
}

fn fake_repopilot() -> &'static str {
    r#"#!/usr/bin/env python3
import json, os, sys
with open(os.environ["FAKE_SCAN_LOG"], "a", encoding="utf-8") as log:
    log.write(sys.argv[0] + "|" + " ".join(sys.argv[1:]) + "\n")
if sys.argv[1:] == ["--version"]:
    print("repopilot " + os.environ.get("FAKE_REPOPILOT_VERSION", "0.18.0"))
    raise SystemExit(0)
if len(sys.argv) > 1 and sys.argv[1] == "scan":
    profile = sys.argv[sys.argv.index("--profile") + 1]
    version = os.environ.get("FAKE_REPOPILOT_VERSION", "0.18.0")
    schema = os.environ.get("FAKE_REPOPILOT_SCHEMA_VERSION", "0.19")
    if profile == "strict":
        version = os.environ.get("FAKE_STRICT_REPOPILOT_VERSION", version)
        schema = os.environ.get("FAKE_STRICT_SCHEMA_VERSION", schema)
    print(json.dumps({
        "schema_version": schema,
        "repopilot_version": version,
        "report": {"kind": "scan", "schema_version": schema, "repopilot_version": version},
        "findings": [],
    }))
    raise SystemExit(0)
print("unexpected repopilot args: " + repr(sys.argv), file=sys.stderr)
raise SystemExit(9)
"#
}

fn stale_repopilot() -> &'static str {
    r#"#!/usr/bin/env python3
import os, sys
with open(os.environ["FAKE_STALE_LOG"], "a", encoding="utf-8") as log:
    log.write("stale invoked\n")
print("stale target binary was used", file=sys.stderr)
raise SystemExit(88)
"#
}
