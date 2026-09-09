use serde_json::Value;
use std::collections::BTreeSet;
use std::fs;
use std::path::Path;

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct SuggestedCheck {
    pub(crate) id: String,
    pub(crate) command: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct InitSuggestions {
    pub(crate) stacks: Vec<String>,
    pub(crate) checks: Vec<SuggestedCheck>,
    pub(crate) critical_paths: Vec<String>,
}

pub(crate) fn render(root: &Path) -> String {
    let suggestions = detect(root);
    let mut output = String::new();

    if suggestions.stacks.is_empty() {
        output.push_str("Detected stack: unknown\n");
    } else {
        output.push_str(&format!(
            "Detected stack: {}\n",
            suggestions.stacks.join(", ")
        ));
    }

    if suggestions.checks.is_empty() {
        output.push_str("No stack-specific verification checks detected.\n");
    } else {
        output.push_str("Suggested verification checks (not run):\n");
        for check in suggestions.checks {
            output.push_str(&format!("  - {}: {}\n", check.id, check.command));
        }
    }

    if suggestions.critical_paths.is_empty() {
        output.push_str("No critical path candidates detected.\n");
    } else {
        output.push_str("Critical path candidates (review before committing):\n");
        for path in suggestions.critical_paths {
            output.push_str(&format!("  - {path}\n"));
        }
    }

    output.push_str("No commands were run; these are deterministic suggestions only.\n");
    output
}

fn detect(root: &Path) -> InitSuggestions {
    let mut stacks = Vec::new();
    let mut checks = Vec::new();

    if root.join("Cargo.toml").is_file() {
        stacks.push("Rust".to_string());
        checks.push(SuggestedCheck {
            id: "rust.test".to_string(),
            command: "cargo test --all".to_string(),
        });
    }

    if root.join("package.json").is_file() {
        stacks.push("Node.js".to_string());
        checks.extend(node_checks(root));
    }

    if has_python_marker(root) {
        stacks.push("Python".to_string());
        if has_pytest_evidence(root) {
            checks.push(SuggestedCheck {
                id: "python.test".to_string(),
                command: "python3 -m pytest -q".to_string(),
            });
        }
    }

    if root.join("go.mod").is_file() {
        stacks.push("Go".to_string());
        checks.push(SuggestedCheck {
            id: "go.test".to_string(),
            command: "go test ./...".to_string(),
        });
    }

    if root.join("pom.xml").is_file() {
        stacks.push("Java/Maven".to_string());
        checks.push(SuggestedCheck {
            id: "maven.test".to_string(),
            command: "mvn test".to_string(),
        });
    }

    if root.join("build.gradle").is_file() || root.join("build.gradle.kts").is_file() {
        stacks.push("Java/Gradle".to_string());
        let command = if root.join("gradlew").is_file() {
            "./gradlew test"
        } else {
            "gradle test"
        };
        checks.push(SuggestedCheck {
            id: "gradle.test".to_string(),
            command: command.to_string(),
        });
    }

    InitSuggestions {
        stacks,
        checks,
        critical_paths: critical_path_candidates(root),
    }
}

fn node_checks(root: &Path) -> Vec<SuggestedCheck> {
    let Ok(contents) = fs::read_to_string(root.join("package.json")) else {
        return Vec::new();
    };
    let Ok(package): Result<Value, _> = serde_json::from_str(&contents) else {
        return Vec::new();
    };
    let Some(scripts) = package.get("scripts").and_then(Value::as_object) else {
        return Vec::new();
    };

    [
        ("test", "node.test", "npm test"),
        ("lint", "node.lint", "npm run lint"),
        ("build", "node.build", "npm run build"),
    ]
    .into_iter()
    .filter(|(script, _, _)| {
        scripts
            .get(*script)
            .and_then(Value::as_str)
            .is_some_and(|command| !command.trim().is_empty())
    })
    .map(|(_, id, command)| SuggestedCheck {
        id: id.to_string(),
        command: command.to_string(),
    })
    .collect()
}

fn has_python_marker(root: &Path) -> bool {
    [
        "pyproject.toml",
        "pytest.ini",
        "setup.cfg",
        "requirements.txt",
    ]
    .into_iter()
    .any(|name| root.join(name).is_file())
}

fn has_pytest_evidence(root: &Path) -> bool {
    [
        "pytest.ini",
        "pyproject.toml",
        "setup.cfg",
        "requirements.txt",
        "requirements-dev.txt",
    ]
    .into_iter()
    .filter_map(|name| fs::read_to_string(root.join(name)).ok())
    .any(|contents| contents.to_ascii_lowercase().contains("pytest"))
}

fn critical_path_candidates(root: &Path) -> Vec<String> {
    const DIRECTORIES: &[(&str, &str)] = &[
        (".github/workflows", ".github/workflows/**"),
        ("config", "config/**"),
        ("db/migrations", "db/migrations/**"),
        ("deploy", "deploy/**"),
        ("infra", "infra/**"),
        ("migrations", "migrations/**"),
        ("security", "security/**"),
        ("src/auth", "src/auth/**"),
        ("src/routes", "src/routes/**"),
        ("src/security", "src/security/**"),
        ("auth", "auth/**"),
    ];
    let mut candidates = BTreeSet::new();

    for (directory, pattern) in DIRECTORIES {
        if root.join(directory).is_dir() {
            candidates.insert((*pattern).to_string());
        }
    }

    if let Ok(entries) = fs::read_dir(root)
        && entries.flatten().any(|entry| {
            entry.file_type().is_ok_and(|file_type| file_type.is_file())
                && entry
                    .file_name()
                    .to_str()
                    .is_some_and(|name| name == ".env" || name.starts_with(".env."))
        })
    {
        candidates.insert(".env*".to_string());
    }

    candidates.into_iter().collect()
}

#[cfg(test)]
mod tests {
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

        assert_eq!(suggestions.stacks, vec!["Node.js"]);
        assert_eq!(
            suggestions.checks,
            vec![
                SuggestedCheck {
                    id: "node.test".to_string(),
                    command: "npm test".to_string(),
                },
                SuggestedCheck {
                    id: "node.build".to_string(),
                    command: "npm run build".to_string(),
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
            vec![".env*", "migrations/**", "src/security/**"]
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

        assert_eq!(suggestions.stacks, vec!["Rust", "Python", "Go"]);
        assert_eq!(
            suggestions.checks,
            vec![
                SuggestedCheck {
                    id: "rust.test".to_string(),
                    command: "cargo test --all".to_string(),
                },
                SuggestedCheck {
                    id: "go.test".to_string(),
                    command: "go test ./...".to_string(),
                },
            ]
        );
    }
}
