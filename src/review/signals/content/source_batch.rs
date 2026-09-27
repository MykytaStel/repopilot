use super::{ReviewSource, post_change_source, source_from};
use crate::review::content_signals::ContentToggles;
use crate::review::diff::{
    BatchedContent, ChangeStatus, ChangedFile, DiffTarget, git_show, git_show_many,
};
use std::path::Path;

pub(in crate::review) fn pre_change_sources(
    repo_root: &Path,
    target: DiffTarget<'_>,
    changed_files: &[ChangedFile],
    toggles: &ContentToggles,
) -> Vec<Option<ReviewSource>> {
    if !toggles.behavioral && !toggles.algorithmic {
        return changed_files.iter().map(|_| None).collect();
    }

    let reference = match target {
        DiffTarget::WorkingTree => "HEAD",
        DiffTarget::Refs { base, .. } | DiffTarget::SinceRef { base } => base,
    };
    let required = changed_files
        .iter()
        .map(|file| !matches!(file.status, ChangeStatus::Added | ChangeStatus::Untracked))
        .collect::<Vec<_>>();
    read_batched_sources(repo_root, reference, changed_files, &required)
}

pub(in crate::review) fn post_change_sources(
    repo_root: &Path,
    target: DiffTarget<'_>,
    changed_files: &[ChangedFile],
    required: &[bool],
) -> Vec<Option<ReviewSource>> {
    debug_assert_eq!(changed_files.len(), required.len());
    match target {
        DiffTarget::Refs { head, .. } => {
            let needed = changed_files
                .iter()
                .zip(required)
                .map(|(file, required)| *required && file.status != ChangeStatus::Deleted)
                .collect::<Vec<_>>();
            read_batched_sources(repo_root, head, changed_files, &needed)
        }
        DiffTarget::WorkingTree | DiffTarget::SinceRef { .. } => changed_files
            .iter()
            .zip(required)
            .map(|(file, required)| {
                (*required)
                    .then(|| post_change_source(repo_root, file, target))
                    .flatten()
            })
            .collect(),
    }
}

fn read_batched_sources(
    repo_root: &Path,
    reference: &str,
    changed_files: &[ChangedFile],
    required: &[bool],
) -> Vec<Option<ReviewSource>> {
    debug_assert_eq!(changed_files.len(), required.len());
    let paths = changed_files
        .iter()
        .zip(required)
        .filter(|(_, required)| **required)
        .map(|(file, _)| file.path_string())
        .collect::<Vec<_>>();
    let mut batched = git_show_many(repo_root, reference, &paths).map(Vec::into_iter);

    changed_files
        .iter()
        .zip(required)
        .map(|(file, required)| {
            if !required {
                return None;
            }
            let path = file.path_string();
            let content = match batched.as_mut().and_then(Iterator::next) {
                Some(BatchedContent::Loaded(content)) => content,
                Some(BatchedContent::Fallback) | None => git_show(repo_root, reference, &path),
            }?;
            Some(source_from(content, &file.path))
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use std::path::PathBuf;
    use std::process::Command;
    use tempfile::TempDir;

    fn git(root: &Path, args: &[&str]) {
        let output = Command::new("git")
            .args(args)
            .current_dir(root)
            .output()
            .expect("run git");
        assert!(
            output.status.success(),
            "git {args:?}: {}",
            String::from_utf8_lossy(&output.stderr)
        );
    }

    #[test]
    fn newline_paths_fall_back_to_the_single_path_reader() {
        let temp = TempDir::new().expect("temporary repository");
        let root = temp.path();
        git(root, &["init", "--quiet"]);
        git(root, &["config", "user.email", "repopilot@example.test"]);
        git(root, &["config", "user.name", "RepoPilot tests"]);
        let path = "src/line\nbreak.rs";
        fs::create_dir_all(root.join("src")).expect("create source directory");
        fs::write(root.join(path), "pub fn before() {}\n").expect("write source");
        git(root, &["add", "--all"]);
        git(root, &["commit", "--quiet", "-m", "base"]);

        let file = ChangedFile {
            path: PathBuf::from(path),
            status: ChangeStatus::Modified,
            ranges: Vec::new(),
            hunks: Vec::new(),
        };
        let loaded = pre_change_sources(
            root,
            DiffTarget::Refs {
                base: "HEAD",
                head: "HEAD",
            },
            &[file],
            &ContentToggles {
                behavioral: true,
                algorithmic: false,
                taint: false,
            },
        );
        assert_eq!(
            loaded[0].as_ref().map(ReviewSource::content),
            Some("pub fn before() {}\n")
        );
    }

    #[test]
    fn ref_post_sources_use_the_selected_head() {
        let temp = TempDir::new().expect("temporary repository");
        let root = temp.path();
        git(root, &["init", "--quiet"]);
        git(root, &["config", "user.email", "repopilot@example.test"]);
        git(root, &["config", "user.name", "RepoPilot tests"]);
        fs::write(root.join("module.rs"), "pub fn before() {}\n").expect("write source");
        git(root, &["add", "--all"]);
        git(root, &["commit", "--quiet", "-m", "base"]);
        let base = Command::new("git")
            .args(["rev-parse", "HEAD"])
            .current_dir(root)
            .output()
            .expect("read base ref");
        fs::write(root.join("module.rs"), "pub fn after() {}\n").expect("update source");
        git(root, &["commit", "--quiet", "-am", "head"]);

        let file = ChangedFile {
            path: PathBuf::from("module.rs"),
            status: ChangeStatus::Modified,
            ranges: Vec::new(),
            hunks: Vec::new(),
        };
        let loaded = post_change_sources(
            root,
            DiffTarget::Refs {
                base: std::str::from_utf8(&base.stdout).expect("utf8 ref").trim(),
                head: "HEAD",
            },
            &[file],
            &[true],
        );
        assert_eq!(
            loaded[0].as_ref().map(ReviewSource::content),
            Some("pub fn after() {}\n")
        );
    }
}
