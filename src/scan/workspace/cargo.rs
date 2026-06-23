use super::WorkspacePackage;
use std::path::Path;

pub(super) fn from_cargo_workspace(root: &Path) -> Vec<WorkspacePackage> {
    let content = match std::fs::read_to_string(root.join("Cargo.toml")) {
        Ok(c) => c,
        Err(_) => return Vec::new(),
    };
    let value: toml::Value = match toml::from_str(&content) {
        Ok(v) => v,
        Err(_) => return Vec::new(),
    };

    let members = value
        .get("workspace")
        .and_then(|w| w.get("members"))
        .and_then(|m| m.as_array())
        .map(|arr| {
            arr.iter()
                .filter_map(|v| v.as_str().map(str::to_string))
                .collect::<Vec<_>>()
        })
        .unwrap_or_default();

    if members.is_empty() {
        return Vec::new();
    }

    let mut packages = Vec::new();
    for member in &members {
        let trimmed = member.trim_end_matches('/');
        if trimmed.contains('*') {
            let prefix = trimmed.trim_end_matches("/*").trim_end_matches("/**");
            let base = root.join(prefix);
            let Ok(entries) = std::fs::read_dir(&base) else {
                continue;
            };
            for entry in entries.filter_map(Result::ok) {
                let path = entry.path();
                if path.is_dir()
                    && path.join("Cargo.toml").is_file()
                    && let Some(name) = package_name_from_path(&path)
                {
                    packages.push(WorkspacePackage {
                        name,
                        root: path,
                        exposes_subpath_exports: false,
                    });
                }
            }
        } else {
            let path = root.join(trimmed);
            if path.join("Cargo.toml").is_file()
                && let Some(name) = package_name_from_path(&path)
            {
                packages.push(WorkspacePackage {
                    name,
                    root: path,
                    exposes_subpath_exports: false,
                });
            }
        }
    }
    packages
}

fn package_name_from_path(path: &Path) -> Option<String> {
    let content = std::fs::read_to_string(path.join("Cargo.toml")).ok()?;
    let value: toml::Value = toml::from_str(&content).ok()?;
    value
        .get("package")
        .and_then(|p| p.get("name"))
        .and_then(|n| n.as_str())
        .map(str::to_string)
        .or_else(|| {
            path.file_name()
                .and_then(|n| n.to_str())
                .map(str::to_string)
        })
}
