//! End-to-end contract for deterministic, non-executing `init` suggestions.

use std::fs;
use std::path::Path;
use std::process::Command;
use tempfile::tempdir;

fn run_init(root: &Path) -> String {
    let output = Command::new(env!("CARGO_BIN_EXE_repopilot"))
        .arg("init")
        .current_dir(root)
        .output()
        .expect("run repopilot init");
    assert!(output.status.success(), "init failed: {:?}", output.status);
    String::from_utf8(output.stdout).expect("init stdout is UTF-8")
}

#[test]
fn init_suggests_rust_checks_and_existing_critical_paths_without_running_them() {
    let temp = tempdir().expect("tempdir");
    fs::write(
        temp.path().join("Cargo.toml"),
        "[package]\nname = \"demo\"\nversion = \"0.1.0\"\n",
    )
    .expect("Cargo.toml");
    fs::create_dir_all(temp.path().join("src/auth")).expect("auth directory");
    fs::write(temp.path().join("src/auth/mod.rs"), "pub fn login() {}\n").expect("auth");
    fs::create_dir_all(temp.path().join("migrations")).expect("migrations directory");
    fs::write(
        temp.path().join("migrations/001_init.sql"),
        "-- migration\n",
    )
    .expect("migration");
    fs::create_dir_all(temp.path().join(".github/workflows")).expect("workflow directory");
    fs::write(temp.path().join(".github/workflows/ci.yml"), "name: CI\n").expect("workflow");

    let output = run_init(temp.path());

    assert!(output.contains("Detected stack: Rust"), "{output}");
    assert!(
        output.contains("cargo test --all (source: Cargo.toml)"),
        "{output}"
    );
    assert!(
        output.contains("Suggested verification checks (not run):"),
        "{output}"
    );
    assert!(
        output.contains("src/auth/** (source: src/auth)"),
        "{output}"
    );
    assert!(
        output.contains("migrations/** (source: migrations)"),
        "{output}"
    );
    assert!(
        output.contains(".github/workflows/** (source: .github/workflows)"),
        "{output}"
    );
    assert!(output.contains("No commands were run"), "{output}");
    assert!(!temp.path().join("verification-ran").exists());
}

#[test]
fn init_suggests_only_declared_npm_scripts_and_preserves_config() {
    let temp = tempdir().expect("tempdir");
    fs::write(
        temp.path().join("package.json"),
        r#"{
  "name": "demo",
  "scripts": { "test": "vitest", "lint": "eslint .", "build": "vite build" }
}
"#,
    )
    .expect("package.json");
    fs::create_dir_all(temp.path().join("src/routes")).expect("routes directory");
    fs::write(temp.path().join("src/routes/index.ts"), "export {}\n").expect("route");
    let config = temp.path().join("repopilot.toml");
    fs::write(&config, "custom = true\n").expect("existing config");

    let output = run_init(temp.path());

    assert!(output.contains("Detected stack: Node.js"), "{output}");
    assert!(
        output.contains("npm test (source: package.json:scripts.test)"),
        "{output}"
    );
    assert!(
        output.contains("npm run lint (source: package.json:scripts.lint)"),
        "{output}"
    );
    assert!(
        output.contains("npm run build (source: package.json:scripts.build)"),
        "{output}"
    );
    assert!(
        output.contains("src/routes/** (source: src/routes)"),
        "{output}"
    );
    assert_eq!(
        fs::read_to_string(config).expect("config"),
        "custom = true\n"
    );
}

#[test]
fn init_reports_python_and_go_provenance_without_claiming_python_without_pytest() {
    let temp = tempdir().expect("tempdir");
    fs::write(
        temp.path().join("pyproject.toml"),
        "[project]\nname = \"demo\"\ndependencies = [\"pytest\"]\n",
    )
    .expect("pyproject");
    fs::write(temp.path().join("go.mod"), "module example.test\n").expect("go.mod");

    let output = run_init(temp.path());

    assert!(
        output.contains("Detected stack: Python (pyproject.toml), Go (go.mod)"),
        "{output}"
    );
    assert!(
        output.contains("python3 -m pytest -q (source: pyproject.toml)"),
        "{output}"
    );
    assert!(
        output.contains("go test ./... (source: go.mod)"),
        "{output}"
    );
    assert!(output.contains("No commands were run"), "{output}");
}

#[test]
fn init_stays_explicitly_unknown_without_stack_or_path_inference() {
    let temp = tempdir().expect("tempdir");
    fs::write(temp.path().join("README.md"), "# Demo\n").expect("readme");

    let output = run_init(temp.path());

    assert!(output.contains("Detected stack: unknown"), "{output}");
    assert!(
        output.contains("No stack-specific verification checks detected."),
        "{output}"
    );
    assert!(
        output.contains("No critical path candidates detected."),
        "{output}"
    );
    assert!(output.contains("No commands were run"), "{output}");
}
