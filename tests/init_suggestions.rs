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
    assert!(output.contains("cargo test --all"), "{output}");
    assert!(
        output.contains("Suggested verification checks (not run):"),
        "{output}"
    );
    assert!(output.contains("src/auth/**"), "{output}");
    assert!(output.contains("migrations/**"), "{output}");
    assert!(output.contains(".github/workflows/**"), "{output}");
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
    assert!(output.contains("npm test"), "{output}");
    assert!(output.contains("npm run lint"), "{output}");
    assert!(output.contains("npm run build"), "{output}");
    assert!(output.contains("src/routes/**"), "{output}");
    assert_eq!(
        fs::read_to_string(config).expect("config"),
        "custom = true\n"
    );
}
