use crate::frameworks::types::DetectedFramework;
use std::path::Path;

pub(crate) mod go;
pub(crate) mod js;
pub(crate) mod python;
mod workspace;

pub use workspace::detect_framework_projects;

pub fn detect_frameworks(root: &Path) -> Vec<DetectedFramework> {
    let mut frameworks = Vec::new();
    for frontend in crate::languages::all_frontends() {
        if let Some(probe) = frontend.framework_probe {
            frameworks.extend(probe(root));
        }
    }
    frameworks.dedup();
    frameworks
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
    use std::path::PathBuf;
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
