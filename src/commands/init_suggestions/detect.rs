use super::{CriticalPathCandidate, DetectedStack, InitSuggestions, SuggestedCheck};
use serde_json::Value;
use std::collections::BTreeSet;
use std::fs;
use std::path::Path;

pub(crate) fn detect(
    root: &Path,
    configured_check_ids: Option<&BTreeSet<String>>,
) -> InitSuggestions {
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

    detect_gradle(root, &mut stacks, &mut checks);

    let config_comparison_available = configured_check_ids.is_some();
    let (checks, configured_check_ids) = omit_configured_checks(checks, configured_check_ids);

    InitSuggestions {
        stacks,
        checks,
        config_comparison_available,
        configured_check_ids,
        critical_paths: critical_path_candidates(root),
    }
}

fn detect_gradle(root: &Path, stacks: &mut Vec<DetectedStack>, checks: &mut Vec<SuggestedCheck>) {
    let source = if root.join("build.gradle").is_file() {
        Some("build.gradle")
    } else if root.join("build.gradle.kts").is_file() {
        Some("build.gradle.kts")
    } else {
        None
    };
    if let Some(source) = source {
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
}

fn omit_configured_checks(
    checks: Vec<SuggestedCheck>,
    configured_check_ids: Option<&BTreeSet<String>>,
) -> (Vec<SuggestedCheck>, Vec<String>) {
    let mut omitted = BTreeSet::new();
    let remaining = checks
        .into_iter()
        .filter(|check| {
            let already_configured =
                configured_check_ids.is_some_and(|ids| ids.contains(&check.id));
            if already_configured {
                omitted.insert(check.id.clone());
            }
            !already_configured
        })
        .collect();
    (remaining, omitted.into_iter().collect())
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
