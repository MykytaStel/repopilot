use std::path::{Path, PathBuf};

mod cargo;
mod go;
mod js;

#[cfg(test)]
mod tests;

#[derive(Debug, Clone)]
pub struct WorkspacePackage {
    pub name: String,
    pub root: PathBuf,
}

/// Detect workspace packages under `root`.
///
/// Checks (in order): npm/yarn workspaces (`package.json`), pnpm workspaces
/// (`pnpm-workspace.yaml`), Cargo workspace (`Cargo.toml`), Go multi-module
/// workspace (`go.work`). Returns the first non-empty list found. Returns an
/// empty vec when the root is not a workspace.
pub fn detect_workspace_packages(root: &Path) -> Vec<WorkspacePackage> {
    let npm = js::from_npm_workspaces(root);
    if !npm.is_empty() {
        return npm;
    }

    let pnpm = js::from_pnpm_workspaces(root);
    if !pnpm.is_empty() {
        return pnpm;
    }

    let cargo = cargo::from_cargo_workspace(root);
    if !cargo.is_empty() {
        return cargo;
    }

    go::from_go_workspace(root)
}

/// Directories directly under `path`. Shared by the manifest parsers that
/// expand `dir/*` globs.
pub(super) fn child_dirs(path: &Path) -> Vec<PathBuf> {
    let Ok(entries) = std::fs::read_dir(path) else {
        return Vec::new();
    };
    entries
        .filter_map(Result::ok)
        .map(|e| e.path())
        .filter(|p| p.is_dir())
        .collect()
}
