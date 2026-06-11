use super::WorkspacePackage;
use std::path::Path;

/// Go multi-module workspace: each directory a `go.work` file `use`s and that
/// carries its own `go.mod` is a workspace package. The package name is the
/// module path declared in that `go.mod` (falling back to the directory name).
///
/// (`src/review/signals/behavioral/dependency.rs` has a parallel `go.work`
/// reader tuned to import-prefix resolution; the two are kept separate because
/// they consume the file for different purposes.)
pub(super) fn from_go_workspace(root: &Path) -> Vec<WorkspacePackage> {
    let content = match std::fs::read_to_string(root.join("go.work")) {
        Ok(c) => c,
        Err(_) => return Vec::new(),
    };

    let mut packages = Vec::new();
    for dir in go_work_uses(&content) {
        let package_root = root.join(&dir);
        if !package_root.join("go.mod").is_file() {
            continue;
        }
        let name = go_module_name(&package_root).unwrap_or_else(|| {
            package_root
                .file_name()
                .unwrap_or_default()
                .to_string_lossy()
                .into_owned()
        });
        packages.push(WorkspacePackage {
            name,
            root: package_root,
        });
    }
    packages
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

fn go_module_name(package_root: &Path) -> Option<String> {
    let content = std::fs::read_to_string(package_root.join("go.mod")).ok()?;
    content
        .lines()
        .find_map(|line| line.trim().strip_prefix("module "))
        .map(|module| module.trim().to_string())
}
