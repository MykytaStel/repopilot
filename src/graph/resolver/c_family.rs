use super::normalize_path;
use std::collections::{BTreeSet, HashSet};
use std::path::{Path, PathBuf};

pub(super) fn resolve_c_include(
    raw: &str,
    from_file: &Path,
    root: &Path,
    known_files: &HashSet<PathBuf>,
) -> Option<PathBuf> {
    let include = Path::new(raw);
    if include.as_os_str().is_empty() || include.is_absolute() {
        return None;
    }
    let root = normalize_path(root);
    let source = normalize_path(from_file);
    let mut candidates = BTreeSet::new();
    if let Some(directory) = source.parent() {
        candidates.insert(normalize_path(&directory.join(include)));
    }
    candidates.insert(normalize_path(&root.join(include)));
    let targets = candidates
        .into_iter()
        .filter(|candidate| candidate.starts_with(&root) && known_files.contains(candidate))
        .collect::<Vec<_>>();
    (targets.len() == 1).then(|| targets[0].clone())
}

pub(super) fn definitive_local_candidates(
    raw: &str,
    from_file: &Path,
    root: &Path,
) -> Option<Vec<PathBuf>> {
    let include = Path::new(raw);
    if include.as_os_str().is_empty() || include.is_absolute() {
        return None;
    }
    let root = normalize_path(root);
    let source = normalize_path(from_file);
    let mut candidates = BTreeSet::new();
    if let Some(directory) = source.parent() {
        candidates.insert(normalize_path(&directory.join(include)));
    }
    candidates.insert(normalize_path(&root.join(include)));
    let candidates = candidates.into_iter().collect::<Vec<_>>();
    candidates
        .iter()
        .all(|candidate| candidate.starts_with(&root))
        .then_some(candidates)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn resolves_unique_includer_relative_or_repository_root_target() {
        let root = Path::new("/repo");
        let local = PathBuf::from("/repo/src/include/types.h");
        let files = HashSet::from([local.clone()]);
        assert_eq!(
            resolve_c_include(
                "include/types.h",
                Path::new("/repo/src/app.c"),
                root,
                &files
            ),
            Some(local)
        );
    }

    #[test]
    fn quoted_include_paths_preserve_spaces_and_unicode() {
        let root = Path::new("/repo");
        let local = PathBuf::from("/repo/src/дані types.h");
        let files = HashSet::from([local.clone()]);
        assert_eq!(
            resolve_c_include("дані types.h", Path::new("/repo/src/app.c"), root, &files),
            Some(local)
        );
    }

    #[test]
    fn preserves_duplicate_candidate_ambiguity_and_rejects_root_escape() {
        let root = Path::new("/repo");
        let files = HashSet::from([
            PathBuf::from("/repo/src/common.h"),
            PathBuf::from("/repo/common.h"),
        ]);
        assert_eq!(
            resolve_c_include("common.h", Path::new("/repo/src/app.cpp"), root, &files),
            None
        );
        assert_eq!(
            resolve_c_include(
                "../../outside.h",
                Path::new("/repo/src/app.c"),
                root,
                &files
            ),
            None
        );
    }

    #[test]
    fn unresolved_candidates_are_bounded_to_repository_root() {
        let root = Path::new("/repo");
        assert!(
            definitive_local_candidates("missing.h", Path::new("/repo/src/app.c"), root).is_some()
        );
        assert!(
            definitive_local_candidates("../../outside.h", Path::new("/repo/src/app.c"), root)
                .is_none()
        );
    }
}
