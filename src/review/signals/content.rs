//! Re-acquiring pre/post-change file content for content-based review signals.
//!
//! By the time the review runs, the scan has dropped per-file content and the
//! parsed syntax trees, and the diff parser keeps only changed-line ranges plus
//! hunk text. The behavioral and algorithmic signals need the *actual source* on
//! each side of the change, so this module re-reads it — from the working tree
//! for an uncommitted review, or from `git show <ref>:<path>` for a ref range —
//! and pairs it with the shared tree-sitter parse view.
//!
//! Nothing here judges the code. It only makes the before/after source available
//! to the detectors, consistent with the "flag, don't prove" stance in `mod.rs`.

use crate::analysis::parse::ParsedFile;
use crate::review::diff::{ChangeStatus, ChangedFile, DiffTarget, git_show};
use crate::scan::language::detect_language;
use std::fs;
use std::path::Path;

/// One side (pre- or post-change) of a changed file's source. Owns the content
/// and detected language label so a borrowed [`ParsedFile`] view can be produced
/// on demand without a second read.
pub struct ReviewSource {
    content: String,
    language_label: Option<String>,
}

impl ReviewSource {
    #[cfg(test)]
    pub(crate) fn new(content: String, language_label: Option<String>) -> Self {
        Self {
            content,
            language_label,
        }
    }

    /// The source text for this side of the change.
    pub fn content(&self) -> &str {
        &self.content
    }

    /// A parse-once view over this source. Yields a tree only for languages with
    /// a tree-sitter grammar; otherwise the view exists but parses to nothing.
    pub fn parsed(&self) -> ParsedFile<'_> {
        ParsedFile::new(&self.content, self.language_label.as_deref())
    }
}

fn source_from(content: String, path: &Path) -> ReviewSource {
    ReviewSource {
        content,
        language_label: detect_language(path).map(str::to_string),
    }
}

/// The file's content *after* the change.
///
/// `None` when the file was deleted (there is no post-change content) or the
/// content cannot be read. For a working-tree review the post-change source is
/// the file on disk; for a ref range it is the blob at `head`.
pub fn post_change_source(
    repo_root: &Path,
    file: &ChangedFile,
    target: DiffTarget<'_>,
) -> Option<ReviewSource> {
    if file.status == ChangeStatus::Deleted {
        return None;
    }

    let content = match target {
        DiffTarget::WorkingTree => fs::read_to_string(repo_root.join(&file.path)).ok()?,
        DiffTarget::Refs { head, .. } => git_show(repo_root, head, &file.path_string())?,
    };

    Some(source_from(content, &file.path))
}

/// The file's content *before* the change.
///
/// `None` when the file is newly added/untracked (there is no pre-change
/// content) or the content cannot be read. For a working-tree review the
/// pre-change source is the committed `HEAD` blob; for a ref range it is `base`.
pub fn pre_change_source(
    repo_root: &Path,
    file: &ChangedFile,
    target: DiffTarget<'_>,
) -> Option<ReviewSource> {
    if matches!(file.status, ChangeStatus::Added | ChangeStatus::Untracked) {
        return None;
    }

    let reference = match target {
        DiffTarget::WorkingTree => "HEAD",
        DiffTarget::Refs { base, .. } => base,
    };
    let content = git_show(repo_root, reference, &file.path_string())?;

    Some(source_from(content, &file.path))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::review::diff::{ChangeStatus, ChangedFile};
    use std::path::PathBuf;
    use std::process::Command;
    use tempfile::TempDir;

    fn git(root: &Path, args: &[&str]) {
        let output = Command::new("git")
            .args(args)
            .current_dir(root)
            .output()
            .expect("failed to run git");
        assert!(
            output.status.success(),
            "git {args:?} failed: {}",
            String::from_utf8_lossy(&output.stderr)
        );
    }

    fn init_repo(root: &Path) {
        git(root, &["init"]);
        git(root, &["config", "user.email", "repopilot@example.invalid"]);
        git(root, &["config", "user.name", "RepoPilot Test"]);
    }

    fn write(root: &Path, relative: &str, content: &str) {
        let path = root.join(relative);
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent).expect("create parent dir");
        }
        fs::write(path, content).expect("write file");
    }

    fn changed(path: &str, status: ChangeStatus) -> ChangedFile {
        ChangedFile {
            path: PathBuf::from(path),
            status,
            ranges: Vec::new(),
            hunks: Vec::new(),
        }
    }

    #[test]
    fn working_tree_reads_worktree_after_and_head_before() {
        let temp = TempDir::new().expect("temp dir");
        let root = temp.path();
        init_repo(root);
        write(root, "src/app.rs", "fn old() {}\n");
        git(root, &["add", "."]);
        git(root, &["commit", "-m", "initial"]);
        write(root, "src/app.rs", "fn updated() {}\n");

        let file = changed("src/app.rs", ChangeStatus::Modified);
        let before = pre_change_source(root, &file, DiffTarget::WorkingTree).expect("before");
        let after = post_change_source(root, &file, DiffTarget::WorkingTree).expect("after");

        assert_eq!(before.content(), "fn old() {}\n");
        assert_eq!(after.content(), "fn updated() {}\n");
        // The detected Rust grammar parses both sides.
        assert!(before.parsed().tree().is_some());
        assert!(after.parsed().tree().is_some());
    }

    #[test]
    fn deleted_file_has_before_but_no_after() {
        let temp = TempDir::new().expect("temp dir");
        let root = temp.path();
        init_repo(root);
        write(root, "src/gone.rs", "fn gone() {}\n");
        git(root, &["add", "."]);
        git(root, &["commit", "-m", "initial"]);
        fs::remove_file(root.join("src/gone.rs")).expect("remove file");

        let file = changed("src/gone.rs", ChangeStatus::Deleted);
        let before = pre_change_source(root, &file, DiffTarget::WorkingTree).expect("before");
        assert_eq!(before.content(), "fn gone() {}\n");
        assert!(post_change_source(root, &file, DiffTarget::WorkingTree).is_none());
    }

    #[test]
    fn added_file_has_after_but_no_before() {
        let temp = TempDir::new().expect("temp dir");
        let root = temp.path();
        init_repo(root);
        write(root, "src/seed.rs", "fn seed() {}\n");
        git(root, &["add", "."]);
        git(root, &["commit", "-m", "initial"]);
        write(root, "src/added.rs", "fn brand_new() {}\n");

        let file = changed("src/added.rs", ChangeStatus::Untracked);
        assert!(pre_change_source(root, &file, DiffTarget::WorkingTree).is_none());
        let after = post_change_source(root, &file, DiffTarget::WorkingTree).expect("after");
        assert_eq!(after.content(), "fn brand_new() {}\n");
    }

    #[test]
    fn ref_range_reads_blobs_at_base_and_head() {
        let temp = TempDir::new().expect("temp dir");
        let root = temp.path();
        init_repo(root);
        write(root, "src/app.rs", "fn v1() {}\n");
        git(root, &["add", "."]);
        git(root, &["commit", "-m", "v1"]);
        write(root, "src/app.rs", "fn v2() {}\n");
        git(root, &["add", "."]);
        git(root, &["commit", "-m", "v2"]);

        let file = changed("src/app.rs", ChangeStatus::Modified);
        let target = DiffTarget::Refs {
            base: "HEAD~1",
            head: "HEAD",
        };
        let before = pre_change_source(root, &file, target).expect("before");
        let after = post_change_source(root, &file, target).expect("after");
        assert_eq!(before.content(), "fn v1() {}\n");
        assert_eq!(after.content(), "fn v2() {}\n");
    }
}
