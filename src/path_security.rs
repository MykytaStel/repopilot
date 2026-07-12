//! Shared root-confinement for paths supplied through agent-facing surfaces.

use std::fs;
use std::path::{Component, Path, PathBuf};

#[derive(Debug, Clone)]
pub struct RootConfinement {
    root: PathBuf,
    root_label: String,
}

impl RootConfinement {
    pub fn new(root: &Path) -> Result<Self, String> {
        Self::named(root, "workspace root")
    }

    pub fn named(root: &Path, root_label: &str) -> Result<Self, String> {
        let root = root.canonicalize().map_err(|error| {
            format!("workspace root {} is unavailable: {error}", root.display())
        })?;
        Ok(Self {
            root,
            root_label: root_label.to_string(),
        })
    }

    pub fn root(&self) -> &Path {
        &self.root
    }

    pub fn resolve_existing(&self, path: &Path, label: &str) -> Result<PathBuf, String> {
        let candidate = self.joined_candidate(path);
        let resolved = candidate
            .canonicalize()
            .map_err(|error| format!("{label} {} is unavailable: {error}", candidate.display()))?;
        self.ensure_within_root(&resolved, label)?;
        Ok(resolved)
    }

    pub fn resolve_allow_missing(&self, path: &Path, label: &str) -> Result<PathBuf, String> {
        let joined = self.joined_candidate(path);
        if joined.exists() {
            return self.resolve_existing(&joined, label);
        }
        let candidate = self.lexical_candidate(&joined, label)?;

        let mut ancestor = candidate.clone();
        while !ancestor.exists() {
            if !ancestor.pop() {
                return Err(format!(
                    "{label} {} has no accessible parent",
                    candidate.display()
                ));
            }
        }
        let canonical_ancestor = fs::canonicalize(&ancestor).map_err(|error| {
            format!(
                "{label} parent {} is unavailable: {error}",
                ancestor.display()
            )
        })?;
        self.ensure_within_root(&canonical_ancestor, label)?;
        let suffix = candidate.strip_prefix(&ancestor).map_err(|_| {
            format!(
                "{label} {} could not be resolved from {}",
                candidate.display(),
                ancestor.display()
            )
        })?;
        let resolved = canonical_ancestor.join(suffix);
        self.ensure_within_root(&resolved, label)?;
        Ok(resolved)
    }

    fn lexical_candidate(&self, path: &Path, label: &str) -> Result<PathBuf, String> {
        let normalized = normalize_lexically(path)
            .ok_or_else(|| format!("{label} {} escapes the filesystem root", path.display()))?;
        self.ensure_within_root(&normalized, label)?;
        Ok(normalized)
    }

    fn joined_candidate(&self, path: &Path) -> PathBuf {
        if path.is_absolute() {
            path.to_path_buf()
        } else {
            self.root.join(path)
        }
    }

    fn ensure_within_root(&self, path: &Path, label: &str) -> Result<(), String> {
        if path.starts_with(&self.root) {
            Ok(())
        } else {
            Err(format!(
                "{label} {} must stay within {} {}",
                path.display(),
                self.root_label,
                self.root.display()
            ))
        }
    }
}

fn normalize_lexically(path: &Path) -> Option<PathBuf> {
    let mut normalized = PathBuf::new();
    for component in path.components() {
        match component {
            Component::Prefix(prefix) => normalized.push(prefix.as_os_str()),
            Component::RootDir => normalized.push(component.as_os_str()),
            Component::CurDir => {}
            Component::ParentDir => {
                if !normalized.pop() {
                    return None;
                }
            }
            Component::Normal(value) => normalized.push(value),
        }
    }
    Some(normalized)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn accepts_existing_and_missing_paths_inside_root() {
        let temp = tempfile::tempdir().expect("tempdir");
        let existing = temp.path().join("src");
        fs::create_dir(&existing).expect("create src");
        let confinement = RootConfinement::new(temp.path()).expect("confinement");

        assert_eq!(
            confinement
                .resolve_existing(Path::new("src"), "path")
                .expect("existing path"),
            existing.canonicalize().expect("canonical src")
        );
        assert_eq!(
            confinement
                .resolve_allow_missing(Path::new("generated/report.json"), "output")
                .expect("missing output"),
            temp.path()
                .canonicalize()
                .expect("canonical root")
                .join("generated/report.json")
        );
    }

    #[test]
    fn rejects_missing_parent_traversal_outside_root() {
        let parent = tempfile::tempdir().expect("parent");
        let root = parent.path().join("workspace");
        fs::create_dir(&root).expect("create root");
        let confinement = RootConfinement::new(&root).expect("confinement");

        let error = confinement
            .resolve_allow_missing(Path::new("../outside/new.json"), "output")
            .expect_err("traversal must fail");

        assert!(error.contains("must stay within workspace root"));
    }

    #[cfg(unix)]
    #[test]
    fn rejects_missing_leaf_beneath_symlink_that_escapes_root() {
        use std::os::unix::fs::symlink;

        let root = tempfile::tempdir().expect("root");
        let outside = tempfile::tempdir().expect("outside");
        symlink(outside.path(), root.path().join("linked")).expect("create symlink");
        let confinement = RootConfinement::new(root.path()).expect("confinement");

        let error = confinement
            .resolve_allow_missing(Path::new("linked/new.json"), "output")
            .expect_err("symlink escape must fail");

        assert!(error.contains("must stay within workspace root"));
    }
}
