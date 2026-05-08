use crate::frameworks::types::DetectedFramework;
use std::path::Path;

pub fn detect_frameworks(root: &Path) -> Vec<DetectedFramework> {
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

/// Extracts a displayable version string from a package.json version field.
/// Returns None for non-version specifiers: workspace:*, file:…, *, or empty.
fn extract_version(s: &str) -> Option<String> {
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
    let stripped = s.trim_start_matches(|c: char| matches!(c, '^' | '~' | '=' | '>'));
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
}
