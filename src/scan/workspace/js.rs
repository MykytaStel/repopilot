use super::{WorkspacePackage, child_dirs};
use std::path::Path;

pub(super) fn from_npm_workspaces(root: &Path) -> Vec<WorkspacePackage> {
    let content = match std::fs::read_to_string(root.join("package.json")) {
        Ok(c) => c,
        Err(_) => return Vec::new(),
    };
    let value: serde_json::Value = match serde_json::from_str(&content) {
        Ok(v) => v,
        Err(_) => return Vec::new(),
    };

    let patterns = workspace_glob_patterns(&value);
    packages_from_patterns(root, &patterns)
}

pub(super) fn from_pnpm_workspaces(root: &Path) -> Vec<WorkspacePackage> {
    let content = match std::fs::read_to_string(root.join("pnpm-workspace.yaml")) {
        Ok(c) => c,
        Err(_) => return Vec::new(),
    };

    let patterns = parse_pnpm_packages_yaml(&content);
    packages_from_patterns(root, &patterns)
}

fn workspace_glob_patterns(value: &serde_json::Value) -> Vec<String> {
    match value.get("workspaces") {
        Some(serde_json::Value::Array(items)) => items
            .iter()
            .filter_map(|i| i.as_str().map(str::to_string))
            .collect(),
        Some(serde_json::Value::Object(obj)) => obj
            .get("packages")
            .and_then(|v| v.as_array())
            .map(|items| {
                items
                    .iter()
                    .filter_map(|i| i.as_str().map(str::to_string))
                    .collect()
            })
            .unwrap_or_default(),
        _ => Vec::new(),
    }
}

fn parse_pnpm_packages_yaml(content: &str) -> Vec<String> {
    let mut patterns = Vec::new();
    let mut in_packages = false;
    for line in content.lines() {
        if line.trim_start().starts_with("packages:") {
            in_packages = true;
            continue;
        }
        if in_packages {
            let trimmed = line.trim();
            if trimmed.is_empty() || (!trimmed.starts_with('-') && !line.starts_with(' ')) {
                break;
            }
            if let Some(stripped) = trimmed.strip_prefix("- ") {
                let val = stripped.trim_matches('\'').trim_matches('"');
                if !val.is_empty() {
                    patterns.push(val.to_string());
                }
            }
        }
    }
    patterns
}

fn packages_from_patterns(root: &Path, patterns: &[String]) -> Vec<WorkspacePackage> {
    let mut packages = Vec::new();
    for pattern in patterns {
        if pattern.contains("node_modules") || pattern.starts_with('!') {
            continue;
        }
        let trimmed = pattern.trim_end_matches('/');
        let dirs = if let Some(prefix) = trimmed
            .strip_suffix("/*")
            .or_else(|| trimmed.strip_suffix("/**"))
        {
            child_dirs(&root.join(prefix))
        } else if trimmed.contains('*') {
            continue;
        } else {
            vec![root.join(trimmed)]
        };

        for dir in dirs {
            if dir.join("package.json").is_file() {
                let name = npm_package_name(&dir).unwrap_or_else(|| {
                    dir.file_name()
                        .unwrap_or_default()
                        .to_string_lossy()
                        .into_owned()
                });
                packages.push(WorkspacePackage { name, root: dir });
            }
        }
    }
    packages
}

fn npm_package_name(path: &Path) -> Option<String> {
    let content = std::fs::read_to_string(path.join("package.json")).ok()?;
    let value: serde_json::Value = serde_json::from_str(&content).ok()?;
    value
        .get("name")
        .and_then(|n| n.as_str())
        .map(str::to_string)
}
