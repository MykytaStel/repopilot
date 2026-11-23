//! Collectors for dependencies already declared in repository manifests.
//!
//! Importing an already-declared crate, npm package, or Go module inside a new
//! or edited file is ordinary code organization, not a newly introduced supply-
//! chain dependency (edits to the manifest itself already trigger a high-tier
//! `supply chain changed` signal).

use std::fs;
use std::path::Path;

/// Cargo dependency names declared in root `Cargo.toml` (`dependencies`,
/// `dev-dependencies`, `build-dependencies`, `[workspace.dependencies]`, and
/// `[target.*.dependencies]`).
pub(super) fn cargo_declared_dependencies(root: &Path) -> Vec<String> {
    let mut names = Vec::new();
    let Ok(content) = fs::read_to_string(root.join("Cargo.toml")) else {
        return names;
    };
    let Ok(value) = toml::from_str::<toml::Value>(&content) else {
        return names;
    };

    collect_cargo_dep_tables(&value, &mut names);
    if let Some(workspace) = value.get("workspace") {
        collect_cargo_dep_tables(workspace, &mut names);
    }
    if let Some(targets) = value.get("target").and_then(toml::Value::as_table) {
        for target in targets.values() {
            collect_cargo_dep_tables(target, &mut names);
        }
    }
    names
}

fn collect_cargo_dep_tables(container: &toml::Value, out: &mut Vec<String>) {
    for section in ["dependencies", "dev-dependencies", "build-dependencies"] {
        let Some(table) = container.get(section).and_then(toml::Value::as_table) else {
            continue;
        };
        for (key, spec) in table {
            out.push(key.clone());
            // When renamed (`serde_yaml = { package = "serde_yaml_ng" }`), the
            // Rust `use` path uses the table key (`serde_yaml`), already added.
            if let Some(pkg) = spec.get("package").and_then(toml::Value::as_str) {
                out.push(pkg.to_string());
            }
        }
    }
}

/// npm package names declared in root `package.json`.
pub(super) fn npm_declared_dependencies(root: &Path) -> Vec<String> {
    let mut names = Vec::new();
    let Ok(content) = fs::read_to_string(root.join("package.json")) else {
        return names;
    };
    let Ok(value) = serde_json::from_str::<serde_json::Value>(&content) else {
        return names;
    };

    for section in [
        "dependencies",
        "devDependencies",
        "peerDependencies",
        "optionalDependencies",
    ] {
        if let Some(obj) = value.get(section).and_then(serde_json::Value::as_object) {
            names.extend(obj.keys().cloned());
        }
    }
    names
}

/// Go module paths declared in `require` statements of root `go.mod`.
pub(super) fn go_declared_dependencies(root: &Path) -> Vec<String> {
    let mut prefixes = Vec::new();
    let Ok(content) = fs::read_to_string(root.join("go.mod")) else {
        return prefixes;
    };
    let mut in_block = false;
    for raw_line in content.lines() {
        let line = raw_line.split("//").next().unwrap_or("").trim();
        if line.is_empty() {
            continue;
        }
        if in_block {
            if line.starts_with(')') {
                in_block = false;
            } else if let Some(module) = line.split_whitespace().next() {
                prefixes.push(module.trim_matches('"').to_string());
            }
            continue;
        }
        let Some(rest) = line.strip_prefix("require") else {
            continue;
        };
        let rest = rest.trim();
        if let Some(after_open) = rest.strip_prefix('(') {
            in_block = true;
            if let Some(module) = after_open.split_whitespace().next()
                && !module.starts_with(')')
            {
                prefixes.push(module.trim_matches('"').to_string());
            }
        } else if let Some(module) = rest.split_whitespace().next() {
            prefixes.push(module.trim_matches('"').to_string());
        }
    }
    prefixes
}

#[cfg(test)]
mod tests {
    use crate::review::diff::{ChangeStatus, ChangedFile, ChangedRange};
    use crate::review::signals::behavioral::{
        BehavioralKind, DependencyContext, detect_behavioral_added,
    };
    use crate::review::signals::content::ReviewSource;
    use std::path::PathBuf;

    #[test]
    fn already_declared_cargo_dependency_is_not_flagged_as_new_import() {
        let dir = tempfile::tempdir().expect("tempdir");
        std::fs::write(
            dir.path().join("Cargo.toml"),
            "[package]\nname = \"demo\"\nversion = \"0.1.0\"\n\n[dependencies]\ntree-sitter = \"0.27\"\nserde_json = \"1\"\n",
        )
        .expect("write Cargo.toml");

        let deps = DependencyContext::from_repo_root(dir.path());
        let file = ChangedFile {
            path: PathBuf::from("src/helper.rs"),
            status: ChangeStatus::Added,
            ranges: vec![ChangedRange { start: 1, end: 3 }],
            hunks: Vec::new(),
        };
        let src = ReviewSource::new(
            "use tree_sitter::Node;\nuse serde_json::Value;\nuse reqwest::Client;\n".to_string(),
            Some("Rust".to_string()),
        );
        let signals: Vec<_> = detect_behavioral_added(&file, &src, &deps)
            .into_iter()
            .filter(|s| s.kind == BehavioralKind::DependencyImportAdded)
            .collect();
        assert_eq!(
            signals.len(),
            1,
            "only undeclared crate should fire: {signals:?}"
        );
        assert!(signals[0].detail.contains("reqwest"));
    }

    #[test]
    fn already_declared_npm_and_go_dependencies_are_recognized() {
        let dir = tempfile::tempdir().expect("tempdir");
        std::fs::write(
            dir.path().join("package.json"),
            r#"{"name":"app","dependencies":{"zod":"^3.0.0"}}"#,
        )
        .expect("write package.json");
        std::fs::write(
            dir.path().join("go.mod"),
            "module example.com/app\n\nrequire (\n\tgithub.com/stretchr/testify v1.9.0\n)\n",
        )
        .expect("write go.mod");

        let deps = DependencyContext::from_repo_root(dir.path());
        assert!(deps.is_known_package("zod"));
        assert!(!deps.is_known_package("axios"));
        assert!(deps.is_known_package("github.com/stretchr/testify/assert"));
        assert!(!deps.is_known_package("github.com/other/pkg"));
    }
}
