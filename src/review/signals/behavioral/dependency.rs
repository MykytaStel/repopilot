//! Workspace-aware discovery of *local* package names and module prefixes for
//! [`super::DependencyContext`].
//!
//! Each collector reads the repo-root manifest and, when it declares a
//! workspace, the manifests of its members — so an import of one local package
//! from another (common in monorepos) is recognized as local rather than
//! reported as a newly added external dependency. A manifest that exists but
//! fails to parse is surfaced once on stderr instead of silently making every
//! import look external.

use std::fs;
use std::path::{Path, PathBuf};

/// Local Cargo package names: the root `[package].name` plus the `[package].name`
/// of every `[workspace].members` entry.
pub(super) fn cargo_local_names(root: &Path) -> Vec<String> {
    let mut names = Vec::new();
    let path = root.join("Cargo.toml");
    let Ok(content) = fs::read_to_string(&path) else {
        return names;
    };
    let value = match toml::from_str::<toml::Value>(&content) {
        Ok(value) => value,
        Err(err) => {
            report_unparseable(&path, &err);
            return names;
        }
    };

    if let Some(name) = cargo_package_name(&value) {
        names.push(name.to_string());
    }
    if let Some(members) = value
        .get("workspace")
        .and_then(|workspace| workspace.get("members"))
        .and_then(toml::Value::as_array)
    {
        for member in members.iter().filter_map(toml::Value::as_str) {
            for dir in expand_member(root, member) {
                if let Ok(content) = fs::read_to_string(dir.join("Cargo.toml"))
                    && let Ok(value) = toml::from_str::<toml::Value>(&content)
                    && let Some(name) = cargo_package_name(&value)
                {
                    names.push(name.to_string());
                }
            }
        }
    }
    names
}

fn cargo_package_name(value: &toml::Value) -> Option<&str> {
    value.get("package")?.get("name")?.as_str()
}

/// Local npm package names: the root `name` plus the `name` of every workspace
/// package (`"workspaces": [...]` or `"workspaces": { "packages": [...] }`).
pub(super) fn npm_local_names(root: &Path) -> Vec<String> {
    let mut names = Vec::new();
    let path = root.join("package.json");
    let Ok(content) = fs::read_to_string(&path) else {
        return names;
    };
    let value = match serde_json::from_str::<serde_json::Value>(&content) {
        Ok(value) => value,
        Err(err) => {
            report_unparseable(&path, &err);
            return names;
        }
    };

    if let Some(name) = value.get("name").and_then(serde_json::Value::as_str) {
        names.push(name.to_string());
    }
    for glob in npm_workspace_globs(&value) {
        for dir in expand_member(root, &glob) {
            if let Ok(content) = fs::read_to_string(dir.join("package.json"))
                && let Ok(value) = serde_json::from_str::<serde_json::Value>(&content)
                && let Some(name) = value.get("name").and_then(serde_json::Value::as_str)
            {
                names.push(name.to_string());
            }
        }
    }
    names
}

fn npm_workspace_globs(value: &serde_json::Value) -> Vec<String> {
    let Some(workspaces) = value.get("workspaces") else {
        return Vec::new();
    };
    workspaces
        .as_array()
        .or_else(|| {
            workspaces
                .get("packages")
                .and_then(serde_json::Value::as_array)
        })
        .map(|globs| {
            globs
                .iter()
                .filter_map(serde_json::Value::as_str)
                .map(str::to_string)
                .collect()
        })
        .unwrap_or_default()
}

/// Local Go module import prefixes: the root `go.mod` module path plus the module
/// path of every directory a `go.work` file `use`s.
pub(super) fn go_local_prefixes(root: &Path) -> Vec<String> {
    let mut prefixes = Vec::new();
    if let Ok(content) = fs::read_to_string(root.join("go.mod"))
        && let Some(module) = go_module(&content)
    {
        prefixes.push(module.to_string());
    }
    if let Ok(content) = fs::read_to_string(root.join("go.work")) {
        for dir in go_work_uses(&content) {
            if let Ok(content) = fs::read_to_string(root.join(&dir).join("go.mod"))
                && let Some(module) = go_module(&content)
            {
                prefixes.push(module.to_string());
            }
        }
    }
    prefixes
}

fn go_module(content: &str) -> Option<&str> {
    content
        .lines()
        .find_map(|line| line.trim().strip_prefix("module "))
        .map(str::trim)
}

/// Directories a `go.work` file `use`s, supporting both the single-line
/// (`use ./dir`) and block (`use ( … )`) forms.
fn go_work_uses(content: &str) -> Vec<String> {
    let mut uses = Vec::new();
    let mut in_block = false;
    for line in content.lines() {
        let line = line.trim();
        if in_block {
            if line.starts_with(')') {
                in_block = false;
            } else if !line.is_empty() {
                uses.push(line.trim_matches('"').to_string());
            }
            continue;
        }
        let Some(rest) = line.strip_prefix("use") else {
            continue;
        };
        let rest = rest.trim();
        if let Some(rest) = rest.strip_prefix('(') {
            in_block = true;
            let rest = rest.trim();
            if !rest.is_empty() {
                uses.push(rest.trim_matches('"').to_string());
            }
        } else if !rest.is_empty() {
            uses.push(rest.trim_matches('"').to_string());
        }
    }
    uses
}

/// Resolve a workspace member entry to concrete directories. Supports an exact
/// path or a single trailing `*`/`**` wildcard (`crates/*`) — the common cases.
fn expand_member(root: &Path, member: &str) -> Vec<PathBuf> {
    let member = member.trim_matches('"');
    if let Some(parent) = member
        .strip_suffix("/**")
        .or_else(|| member.strip_suffix("/*"))
    {
        let Ok(entries) = fs::read_dir(root.join(parent)) else {
            return Vec::new();
        };
        return entries
            .flatten()
            .map(|entry| entry.path())
            .filter(|path| path.is_dir())
            .collect();
    }
    vec![root.join(member)]
}

fn report_unparseable(path: &Path, err: &dyn std::fmt::Display) {
    eprintln!(
        "repopilot: ignoring unparseable manifest {} ({err}); its imports may look external",
        path.display()
    );
}

#[cfg(test)]
mod tests {
    use super::*;

    fn write(path: &Path, content: &str) {
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent).expect("create dirs");
        }
        fs::write(path, content).expect("write file");
    }

    #[test]
    fn cargo_workspace_members_are_local() {
        let temp = tempfile::tempdir().expect("temp repo");
        let root = temp.path();
        write(
            &root.join("Cargo.toml"),
            "[workspace]\nmembers = [\"crates/*\", \"tools/cli\"]\n",
        );
        write(
            &root.join("crates/engine/Cargo.toml"),
            "[package]\nname = \"acme-engine\"\nversion = \"0.1.0\"\n",
        );
        write(
            &root.join("tools/cli/Cargo.toml"),
            "[package]\nname = \"acme-cli\"\nversion = \"0.1.0\"\n",
        );

        let names = cargo_local_names(root);
        assert!(names.contains(&"acme-engine".to_string()), "{names:?}");
        assert!(names.contains(&"acme-cli".to_string()), "{names:?}");
    }

    #[test]
    fn npm_workspaces_array_and_object_forms_are_local() {
        let temp = tempfile::tempdir().expect("temp repo");
        let root = temp.path();
        write(
            &root.join("package.json"),
            "{\"name\":\"@acme/root\",\"workspaces\":[\"packages/*\"]}",
        );
        write(
            &root.join("packages/ui/package.json"),
            "{\"name\":\"@acme/ui\"}",
        );
        let names = npm_local_names(root);
        assert!(names.contains(&"@acme/root".to_string()), "{names:?}");
        assert!(names.contains(&"@acme/ui".to_string()), "{names:?}");
    }

    #[test]
    fn go_work_modules_are_local_prefixes() {
        let temp = tempfile::tempdir().expect("temp repo");
        let root = temp.path();
        write(
            &root.join("go.mod"),
            "module github.com/acme/app\n\ngo 1.22\n",
        );
        write(&root.join("go.work"), "go 1.22\n\nuse (\n\t./svc\n)\n");
        write(
            &root.join("svc/go.mod"),
            "module github.com/acme/svc\n\ngo 1.22\n",
        );
        let prefixes = go_local_prefixes(root);
        assert!(
            prefixes.contains(&"github.com/acme/app".to_string()),
            "{prefixes:?}"
        );
        assert!(
            prefixes.contains(&"github.com/acme/svc".to_string()),
            "{prefixes:?}"
        );
    }

    #[test]
    fn unparseable_cargo_manifest_yields_no_names() {
        let temp = tempfile::tempdir().expect("temp repo");
        let root = temp.path();
        write(&root.join("Cargo.toml"), "this is not = valid = toml [[[");
        assert!(cargo_local_names(root).is_empty());
    }
}
