use serde_json::Value;
use std::collections::BTreeSet;
use std::fs;
use std::path::Path;

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct DetectedStack {
    pub(crate) name: String,
    pub(crate) source: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct SuggestedCheck {
    pub(crate) id: String,
    pub(crate) command: String,
    pub(crate) source: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct CriticalPathCandidate {
    pub(crate) pattern: String,
    pub(crate) source: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct InitSuggestions {
    pub(crate) stacks: Vec<DetectedStack>,
    pub(crate) checks: Vec<SuggestedCheck>,
    pub(crate) critical_paths: Vec<CriticalPathCandidate>,
}

pub(crate) fn render(root: &Path) -> String {
    let suggestions = detect(root);
    let mut output = String::new();

    if suggestions.stacks.is_empty() {
        output.push_str("Detected stack: unknown\n");
    } else {
        output.push_str(&format!(
            "Detected stack: {}\n",
            suggestions
                .stacks
                .iter()
                .map(|stack| format!("{} ({})", stack.name, stack.source))
                .collect::<Vec<_>>()
                .join(", ")
        ));
    }

    if suggestions.checks.is_empty() {
        output.push_str("No stack-specific verification checks detected.\n");
    } else {
        output.push_str("Suggested verification checks (not run):\n");
        for check in suggestions.checks {
            output.push_str(&format!(
                "  - {}: {} (source: {})\n",
                check.id, check.command, check.source
            ));
        }
    }

    if suggestions.critical_paths.is_empty() {
        output.push_str("No critical path candidates detected.\n");
    } else {
        output.push_str("Critical path candidates (review before committing):\n");
        for path in suggestions.critical_paths {
            output.push_str(&format!("  - {} (source: {})\n", path.pattern, path.source));
        }
    }

    output.push_str("No commands were run; these are deterministic suggestions only.\n");
    output
}

fn detect(root: &Path) -> InitSuggestions {
    let mut stacks = Vec::new();
    let mut checks = Vec::new();

    if root.join("Cargo.toml").is_file() {
        stacks.push(DetectedStack {
            name: "Rust".to_string(),
            source: "Cargo.toml".to_string(),
        });
        checks.push(SuggestedCheck {
            id: "rust.test".to_string(),
            command: "cargo test --all".to_string(),
            source: "Cargo.toml".to_string(),
        });
    }

    if root.join("package.json").is_file() {
        stacks.push(DetectedStack {
            name: "Node.js".to_string(),
            source: "package.json".to_string(),
        });
        checks.extend(node_checks(root));
    }

    if let Some(source) = python_marker(root) {
        stacks.push(DetectedStack {
            name: "Python".to_string(),
            source,
        });
        if let Some(source) = pytest_evidence(root) {
            checks.push(SuggestedCheck {
                id: "python.test".to_string(),
                command: "python3 -m pytest -q".to_string(),
                source,
            });
        }
    }

    if root.join("go.mod").is_file() {
        stacks.push(DetectedStack {
            name: "Go".to_string(),
            source: "go.mod".to_string(),
        });
        checks.push(SuggestedCheck {
            id: "go.test".to_string(),
            command: "go test ./...".to_string(),
            source: "go.mod".to_string(),
        });
    }

    if root.join("pom.xml").is_file() {
        stacks.push(DetectedStack {
            name: "Java/Maven".to_string(),
            source: "pom.xml".to_string(),
        });
        checks.push(SuggestedCheck {
            id: "maven.test".to_string(),
            command: "mvn test".to_string(),
            source: "pom.xml".to_string(),
        });
    }

    let gradle_source = if root.join("build.gradle").is_file() {
        Some("build.gradle")
    } else if root.join("build.gradle.kts").is_file() {
        Some("build.gradle.kts")
    } else {
        None
    };
    if let Some(source) = gradle_source {
        stacks.push(DetectedStack {
            name: "Java/Gradle".to_string(),
            source: source.to_string(),
        });
        let command = if root.join("gradlew").is_file() {
            "./gradlew test"
        } else {
            "gradle test"
        };
        checks.push(SuggestedCheck {
            id: "gradle.test".to_string(),
            command: command.to_string(),
            source: source.to_string(),
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
    .map(|(script, id, command)| SuggestedCheck {
        id: id.to_string(),
        command: command.to_string(),
        source: format!("package.json:scripts.{script}"),
    })
    .collect()
}

fn python_marker(root: &Path) -> Option<String> {
    [
        "pyproject.toml",
        "pytest.ini",
        "setup.cfg",
        "requirements.txt",
    ]
    .into_iter()
    .find(|name| root.join(name).is_file())
    .map(str::to_string)
}

fn pytest_evidence(root: &Path) -> Option<String> {
    [
        "pytest.ini",
        "pyproject.toml",
        "setup.cfg",
        "requirements.txt",
        "requirements-dev.txt",
    ]
    .into_iter()
    .find(|name| {
        fs::read_to_string(root.join(name))
            .ok()
            .is_some_and(|contents| contents.to_ascii_lowercase().contains("pytest"))
    })
    .map(str::to_string)
}

fn critical_path_candidates(root: &Path) -> Vec<CriticalPathCandidate> {
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
            candidates.insert(((*pattern).to_string(), (*directory).to_string()));
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
        candidates.insert((".env*".to_string(), "root .env* marker".to_string()));
    }

    candidates
        .into_iter()
        .map(|(pattern, source)| CriticalPathCandidate { pattern, source })
        .collect()
}

#[cfg(test)]
mod tests;
