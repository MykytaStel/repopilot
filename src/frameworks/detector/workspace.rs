use crate::frameworks::detector::detect_frameworks;
use crate::frameworks::react_native::detect_react_native_architecture;
use crate::frameworks::types::{DetectedFramework, FrameworkProject};
use std::collections::BTreeSet;
use std::path::{Path, PathBuf};

pub fn detect_framework_projects(root: &Path) -> Vec<FrameworkProject> {
    let mut projects = Vec::new();
    let mut seen = BTreeSet::new();

    push_framework_project(root, root, &mut projects, &mut seen);

    for workspace in workspace_package_paths(root) {
        push_framework_project(root, &workspace, &mut projects, &mut seen);
    }

    projects
}

fn push_framework_project(
    root: &Path,
    project_path: &Path,
    projects: &mut Vec<FrameworkProject>,
    seen: &mut BTreeSet<PathBuf>,
) {
    let normalized = project_path
        .strip_prefix(root)
        .map(|p| {
            if p.as_os_str().is_empty() {
                PathBuf::from(".")
            } else {
                p.to_path_buf()
            }
        })
        .unwrap_or_else(|_| project_path.to_path_buf());

    if !seen.insert(normalized.clone()) {
        return;
    }

    let frameworks = detect_frameworks(project_path);
    if frameworks.is_empty() {
        return;
    }

    let react_native = frameworks
        .iter()
        .any(|f| matches!(f, DetectedFramework::ReactNative { .. }))
        .then(|| detect_react_native_architecture(project_path))
        .filter(|profile| profile.detected);

    projects.push(FrameworkProject {
        path: normalized,
        frameworks,
        react_native,
    });
}

fn workspace_package_paths(root: &Path) -> Vec<PathBuf> {
    let pkg_path = root.join("package.json");
    let content = match std::fs::read_to_string(&pkg_path) {
        Ok(c) => c,
        Err(_) => return Vec::new(),
    };
    let value: serde_json::Value = match serde_json::from_str(&content) {
        Ok(v) => v,
        Err(_) => return Vec::new(),
    };

    workspace_patterns(&value)
        .into_iter()
        .flat_map(|pattern| expand_workspace_pattern(root, &pattern))
        .filter(|path| path.join("package.json").is_file())
        .collect()
}

fn workspace_patterns(value: &serde_json::Value) -> Vec<String> {
    match value.get("workspaces") {
        Some(serde_json::Value::Array(items)) => items
            .iter()
            .filter_map(|item| item.as_str().map(str::to_string))
            .collect(),
        Some(serde_json::Value::Object(obj)) => obj
            .get("packages")
            .and_then(|v| v.as_array())
            .map(|items| {
                items
                    .iter()
                    .filter_map(|item| item.as_str().map(str::to_string))
                    .collect()
            })
            .unwrap_or_default(),
        _ => Vec::new(),
    }
}

fn expand_workspace_pattern(root: &Path, pattern: &str) -> Vec<PathBuf> {
    if pattern.contains("node_modules") || pattern.starts_with('!') {
        return Vec::new();
    }

    let trimmed = pattern.trim_end_matches('/');
    if let Some(prefix) = trimmed.strip_suffix("/*") {
        let base = root.join(prefix);
        return child_dirs(&base);
    }
    if let Some(prefix) = trimmed.strip_suffix("/**") {
        let base = root.join(prefix);
        return child_dirs(&base);
    }
    if trimmed.contains('*') {
        return Vec::new();
    }

    vec![root.join(trimmed)]
}

fn child_dirs(path: &Path) -> Vec<PathBuf> {
    let Ok(entries) = std::fs::read_dir(path) else {
        return Vec::new();
    };

    entries
        .filter_map(Result::ok)
        .map(|entry| entry.path())
        .filter(|path| path.is_dir())
        .collect()
}
