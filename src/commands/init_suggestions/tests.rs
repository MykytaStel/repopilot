use super::*;
use std::fs;
use tempfile::tempdir;

#[test]
fn node_proposals_use_only_declared_scripts() {
    let temp = tempdir().expect("temp dir");
    fs::write(
        temp.path().join("package.json"),
        r#"{"scripts":{"test":"vitest","build":"vite build"}}"#,
    )
    .expect("package");

    let suggestions = detect(temp.path());

    assert_eq!(
        suggestions.stacks,
        vec![DetectedStack {
            name: "Node.js".to_string(),
            source: "package.json".to_string(),
        }]
    );
    assert_eq!(
        suggestions.checks,
        vec![
            SuggestedCheck {
                id: "node.test".to_string(),
                command: "npm test".to_string(),
                source: "package.json:scripts.test".to_string(),
            },
            SuggestedCheck {
                id: "node.build".to_string(),
                command: "npm run build".to_string(),
                source: "package.json:scripts.build".to_string(),
            },
        ]
    );
}

#[test]
fn critical_paths_are_existing_and_sorted() {
    let temp = tempdir().expect("temp dir");
    fs::create_dir_all(temp.path().join("src/security")).expect("security");
    fs::create_dir_all(temp.path().join("migrations")).expect("migrations");
    fs::write(temp.path().join(".env.local"), "SECRET=redacted\n").expect("env");

    let suggestions = detect(temp.path());

    assert_eq!(
        suggestions.critical_paths,
        vec![
            CriticalPathCandidate {
                pattern: ".env*".to_string(),
                source: "root .env* marker".to_string(),
            },
            CriticalPathCandidate {
                pattern: "migrations/**".to_string(),
                source: "migrations".to_string(),
            },
            CriticalPathCandidate {
                pattern: "src/security/**".to_string(),
                source: "src/security".to_string(),
            },
        ]
    );
}

#[test]
fn stack_markers_are_deterministic_and_python_requires_pytest_evidence() {
    let temp = tempdir().expect("temp dir");
    fs::write(
        temp.path().join("Cargo.toml"),
        "[package]\nname = \"demo\"\n",
    )
    .expect("cargo");
    fs::write(temp.path().join("go.mod"), "module example.test\n").expect("go");
    fs::write(
        temp.path().join("pyproject.toml"),
        "[project]\nname = \"demo\"\n",
    )
    .expect("python");

    let suggestions = detect(temp.path());

    assert_eq!(
        suggestions.stacks,
        vec![
            DetectedStack {
                name: "Rust".to_string(),
                source: "Cargo.toml".to_string(),
            },
            DetectedStack {
                name: "Python".to_string(),
                source: "pyproject.toml".to_string(),
            },
            DetectedStack {
                name: "Go".to_string(),
                source: "go.mod".to_string(),
            },
        ]
    );
    assert_eq!(
        suggestions.checks,
        vec![
            SuggestedCheck {
                id: "rust.test".to_string(),
                command: "cargo test --all".to_string(),
                source: "Cargo.toml".to_string(),
            },
            SuggestedCheck {
                id: "go.test".to_string(),
                command: "go test ./...".to_string(),
                source: "go.mod".to_string(),
            },
        ]
    );
}
