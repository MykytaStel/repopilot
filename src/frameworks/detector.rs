use crate::frameworks::react_native::detect_react_native_architecture;
use crate::frameworks::types::{DetectedFramework, FrameworkProject};
use std::collections::BTreeSet;
use std::path::{Path, PathBuf};

pub fn detect_frameworks(root: &Path) -> Vec<DetectedFramework> {
    let mut frameworks = detect_js_frameworks(root);
    frameworks.extend(detect_python_frameworks(root));
    frameworks.extend(detect_go_frameworks(root));
    frameworks
}

fn detect_js_frameworks(root: &Path) -> Vec<DetectedFramework> {
    let pkg_path = root.join("package.json");
    let content = match std::fs::read_to_string(&pkg_path) {
        Ok(c) => c,
        Err(_) => return vec![],
    };
    let value: serde_json::Value = match serde_json::from_str(&content) {
        Ok(v) => v,
        Err(_) => return vec![],
    };

    let mut deps = serde_json::Map::new();
    for key in ["dependencies", "devDependencies"] {
        if let Some(obj) = value.get(key).and_then(|v| v.as_object()) {
            for (k, v) in obj {
                deps.entry(k.clone()).or_insert_with(|| v.clone());
            }
        }
    }

    let version_of = |pkg: &str| -> Option<String> {
        deps.get(pkg)
            .and_then(|v| v.as_str())
            .and_then(extract_version)
    };

    let mut frameworks = Vec::new();

    if deps.contains_key("react-native") {
        frameworks.push(DetectedFramework::ReactNative {
            version: version_of("react-native"),
        });
    }
    if deps.contains_key("expo") {
        frameworks.push(DetectedFramework::Expo {
            version: version_of("expo"),
        });
    }
    if deps.contains_key("next") {
        frameworks.push(DetectedFramework::NextJs {
            version: version_of("next"),
        });
    }
    if deps.contains_key("react") {
        frameworks.push(DetectedFramework::React {
            version: version_of("react"),
        });
    }
    if deps.contains_key("vue") {
        frameworks.push(DetectedFramework::Vue {
            version: version_of("vue"),
        });
    }
    if deps.contains_key("@angular/core") {
        frameworks.push(DetectedFramework::Angular {
            version: version_of("@angular/core"),
        });
    }
    if deps.contains_key("svelte") {
        frameworks.push(DetectedFramework::Svelte {
            version: version_of("svelte"),
        });
    }
    if deps.contains_key("@nestjs/core") {
        frameworks.push(DetectedFramework::NestJs {
            version: version_of("@nestjs/core"),
        });
    }
    if deps.contains_key("express") {
        frameworks.push(DetectedFramework::Express {
            version: version_of("express"),
        });
    }

    frameworks
}

fn detect_python_frameworks(root: &Path) -> Vec<DetectedFramework> {
    let content = match std::fs::read_to_string(root.join("requirements.txt")) {
        Ok(c) => c,
        Err(_) => return vec![],
    };

    let mut frameworks = Vec::new();

    for line in content.lines() {
        let line = line.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        // Extract package name: stop at ==, >=, <=, !=, ~=, [, space, #, @
        let name_end = line
            .find(|c: char| ['=', '>', '<', '!', '~', '[', ' ', '#', '@'].contains(&c))
            .unwrap_or(line.len());
        let pkg = line[..name_end].trim().to_lowercase();
        // Extract pinned version after ==, if present
        let version = line
            .find("==")
            .map(|pos| {
                line[pos + 2..]
                    .split_whitespace()
                    .next()
                    .unwrap_or("")
                    .to_string()
            })
            .filter(|v| !v.is_empty());

        match pkg.as_str() {
            "django" => frameworks.push(DetectedFramework::Django { version }),
            "flask" => frameworks.push(DetectedFramework::Flask { version }),
            "fastapi" => frameworks.push(DetectedFramework::FastApi { version }),
            _ => {}
        }
    }

    frameworks
}

fn detect_go_frameworks(root: &Path) -> Vec<DetectedFramework> {
    let content = match std::fs::read_to_string(root.join("go.mod")) {
        Ok(c) => c,
        Err(_) => return vec![],
    };

    let mut frameworks = Vec::new();

    for line in content.lines() {
        let trimmed = line.trim();
        if trimmed.starts_with("//") {
            continue;
        }
        // Strip optional "require " prefix, then parse "module/path vX.Y.Z"
        let entry = trimmed.trim_start_matches("require ").trim();
        let version = entry
            .split_whitespace()
            .nth(1)
            .map(|v| v.trim_start_matches('v').to_string())
            .filter(|v| !v.is_empty());

        if trimmed.contains("gin-gonic/gin") {
            frameworks.push(DetectedFramework::Gin { version });
        } else if trimmed.contains("labstack/echo") {
            frameworks.push(DetectedFramework::Echo { version });
        } else if trimmed.contains("gofiber/fiber") {
            frameworks.push(DetectedFramework::Fiber { version });
        }
    }

    frameworks
}

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

/// Extracts a displayable version string from a package.json version field.
/// Returns None for non-version specifiers: workspace:*, file:…, *, or empty.
pub(crate) fn extract_version(s: &str) -> Option<String> {
    if s.is_empty()
        || s == "*"
        || s.starts_with("workspace:")
        || s.starts_with("file:")
        || s.starts_with("link:")
        || s.starts_with("git+")
        || s.starts_with("github:")
        || s.starts_with("http")
    {
        return None;
    }
    let stripped = s.trim_start_matches(['^', '~', '=', '>']);
    Some(stripped.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;
    use tempfile::tempdir;

    #[test]
    fn detects_react_native_and_expo() {
        let dir = tempdir().unwrap();
        let pkg = dir.path().join("package.json");
        let mut f = std::fs::File::create(&pkg).unwrap();
        write!(
            f,
            r#"{{"dependencies": {{"react-native": "^0.74.0", "expo": "~51.0.0", "react": "18.2.0"}}}}"#
        )
        .unwrap();

        let frameworks = detect_frameworks(dir.path());
        assert!(frameworks.iter().any(
            |f| matches!(f, DetectedFramework::ReactNative { version: Some(v) } if v == "0.74.0")
        ));
        assert!(
            frameworks.iter().any(
                |f| matches!(f, DetectedFramework::Expo { version: Some(v) } if v == "51.0.0")
            )
        );
        assert!(
            frameworks
                .iter()
                .any(|f| matches!(f, DetectedFramework::React { .. }))
        );
    }

    #[test]
    fn returns_empty_without_package_json() {
        let dir = tempdir().unwrap();
        assert!(detect_frameworks(dir.path()).is_empty());
    }

    #[test]
    fn workspace_and_file_refs_produce_no_version() {
        let dir = tempdir().unwrap();
        let pkg = dir.path().join("package.json");
        let mut f = std::fs::File::create(&pkg).unwrap();
        write!(
            f,
            r#"{{"dependencies": {{"react-native": "workspace:*", "react": "file:../react"}}}}"#
        )
        .unwrap();

        let frameworks = detect_frameworks(dir.path());
        assert!(
            frameworks
                .iter()
                .any(|f| matches!(f, DetectedFramework::ReactNative { version: None }))
        );
        assert!(
            frameworks
                .iter()
                .any(|f| matches!(f, DetectedFramework::React { version: None }))
        );
    }

    #[test]
    fn detects_nextjs() {
        let dir = tempdir().unwrap();
        let pkg = dir.path().join("package.json");
        let mut f = std::fs::File::create(&pkg).unwrap();
        write!(
            f,
            r#"{{"dependencies": {{"next": "14.0.0", "react": "18.0.0"}}}}"#
        )
        .unwrap();

        let frameworks = detect_frameworks(dir.path());
        assert!(
            frameworks
                .iter()
                .any(|f| matches!(f, DetectedFramework::NextJs { .. }))
        );
        assert!(
            frameworks
                .iter()
                .any(|f| matches!(f, DetectedFramework::React { .. }))
        );
    }

    #[test]
    fn detects_framework_projects_from_workspaces() {
        let dir = tempdir().unwrap();
        let pkg = dir.path().join("package.json");
        let mut f = std::fs::File::create(&pkg).unwrap();
        write!(f, r#"{{"workspaces": ["apps/*"]}}"#).unwrap();

        std::fs::create_dir_all(dir.path().join("apps/mobile")).unwrap();
        let mut mobile =
            std::fs::File::create(dir.path().join("apps/mobile/package.json")).unwrap();
        write!(
            mobile,
            r#"{{"dependencies": {{"react-native": "0.76.0", "expo": "53.0.0"}}}}"#
        )
        .unwrap();

        let projects = detect_framework_projects(dir.path());

        assert_eq!(projects.len(), 1);
        assert_eq!(projects[0].path, PathBuf::from("apps/mobile"));
        assert!(
            projects[0]
                .frameworks
                .iter()
                .any(|f| matches!(f, DetectedFramework::ReactNative { .. }))
        );
        assert!(projects[0].react_native.is_some());
    }
}
