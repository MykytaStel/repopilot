use crate::config::model::RepoPilotConfig;
use crate::findings::visibility::FindingVisibilityProfile;
use crate::scan::config::ScanConfig;
use ignore::WalkBuilder;
use sha2::{Digest, Sha256};
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

const REVISION_SCHEMA: &str = "analysis-session-v1";
const CACHE_DIR: &str = ".repopilot/cache";

/// Immutable inputs shared by one RepoPilot analysis pass.
///
/// The session keeps scan/review configuration and workspace identity together so
/// command, MCP, and later cached analysis paths cannot accidentally combine
/// values captured at different points in time.
#[derive(Debug, Clone)]
pub struct AnalysisSession {
    analysis_path: PathBuf,
    workspace_root: PathBuf,
    revision: WorkspaceRevision,
    repo_config: RepoPilotConfig,
    scan_config: ScanConfig,
    visibility_profile: FindingVisibilityProfile,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WorkspaceRevision {
    id: String,
}

impl WorkspaceRevision {
    pub fn capture(path: &Path) -> Self {
        let workspace_root = resolve_workspace_root(path);
        capture_workspace_revision(&workspace_root)
    }

    pub fn id(&self) -> &str {
        &self.id
    }
}

impl AnalysisSession {
    pub fn new(
        analysis_path: PathBuf,
        repo_config: RepoPilotConfig,
        scan_config: ScanConfig,
        visibility_profile: FindingVisibilityProfile,
    ) -> Self {
        let workspace_root = resolve_workspace_root(&analysis_path);
        let revision = capture_workspace_revision(&workspace_root);

        Self {
            analysis_path,
            workspace_root,
            revision,
            repo_config,
            scan_config,
            visibility_profile,
        }
    }

    pub fn analysis_path(&self) -> &Path {
        &self.analysis_path
    }

    pub fn workspace_root(&self) -> &Path {
        &self.workspace_root
    }

    pub fn revision(&self) -> &WorkspaceRevision {
        &self.revision
    }

    pub fn repo_config(&self) -> &RepoPilotConfig {
        &self.repo_config
    }

    pub fn scan_config(&self) -> &ScanConfig {
        &self.scan_config
    }

    pub fn visibility_profile(&self) -> FindingVisibilityProfile {
        self.visibility_profile
    }
}

fn resolve_workspace_root(path: &Path) -> PathBuf {
    let command_dir = git_command_dir(path);
    if let Some(root) = git_text(command_dir, &["rev-parse", "--show-toplevel"]) {
        let root = PathBuf::from(root);
        return fs::canonicalize(&root).unwrap_or(root);
    }

    fs::canonicalize(command_dir).unwrap_or_else(|_| command_dir.to_path_buf())
}

fn capture_workspace_revision(workspace_root: &Path) -> WorkspaceRevision {
    let mut hasher = Sha256::new();
    hasher.update(REVISION_SCHEMA.as_bytes());
    hasher.update(b"\nroot:");
    hasher.update(workspace_root.to_string_lossy().as_bytes());

    if let Some(head) = git_text(workspace_root, &["rev-parse", "--verify", "HEAD"]) {
        hasher.update(b"\nsource:git\nhead:");
        hasher.update(head.as_bytes());

        match git_bytes(workspace_root, &["status", "--porcelain=v1", "-z", "-uall"]) {
            Some(status) => {
                let mut entries = status_entries(&status);
                entries.sort_by(|left, right| {
                    (&left.path, &left.status, &left.old_path).cmp(&(
                        &right.path,
                        &right.status,
                        &right.old_path,
                    ))
                });
                hash_status_entries(&mut hasher, workspace_root, entries);
            }
            None => hasher.update(b"\nstatus:unavailable"),
        }
    } else {
        hasher.update(b"\nsource:filesystem");
        hash_filesystem(&mut hasher, workspace_root);
    }

    WorkspaceRevision {
        id: hex(&hasher.finalize()),
    }
}

fn hash_status_entries(hasher: &mut Sha256, workspace_root: &Path, entries: Vec<StatusEntry>) {
    for entry in entries {
        if is_cache_path(&entry.path) {
            continue;
        }

        hasher.update(b"\nstatus:");
        hasher.update(entry.status.as_bytes());
        hasher.update(b" ");
        hasher.update(entry.path.as_bytes());
        if let Some(old_path) = &entry.old_path {
            hasher.update(b" <- ");
            hasher.update(old_path.as_bytes());
        }

        if let Ok(bytes) = fs::read(workspace_root.join(&entry.path)) {
            hasher.update(b"\nfile:");
            hasher.update(entry.path.as_bytes());
            hasher.update(b"\n");
            hasher.update(bytes);
        }
    }
}

fn hash_filesystem(hasher: &mut Sha256, workspace_root: &Path) {
    let root = workspace_root.to_path_buf();
    let mut builder = WalkBuilder::new(workspace_root);
    builder
        .hidden(false)
        .git_ignore(true)
        .git_global(true)
        .git_exclude(true)
        .filter_entry(move |entry| entry.path() == root || !is_internal_path(entry.path(), &root));
    let mut files = builder
        .build()
        .filter_map(Result::ok)
        .filter(|entry| entry.file_type().is_some_and(|kind| kind.is_file()))
        .map(|entry| entry.into_path())
        .collect::<Vec<_>>();
    files.sort();

    for path in files {
        let relative = path.strip_prefix(workspace_root).unwrap_or(&path);
        hasher.update(b"\nfile:");
        hasher.update(relative.to_string_lossy().as_bytes());
        if let Ok(bytes) = fs::read(&path) {
            hasher.update(b"\n");
            hasher.update(bytes);
        }
    }
}

fn is_internal_path(path: &Path, root: &Path) -> bool {
    let relative = path
        .strip_prefix(root)
        .unwrap_or(path)
        .to_string_lossy()
        .replace('\\', "/");
    relative == ".git"
        || relative.starts_with(".git/")
        || relative == CACHE_DIR
        || relative.starts_with(&format!("{CACHE_DIR}/"))
}

fn git_text(path: &Path, args: &[&str]) -> Option<String> {
    let output = git_bytes(path, args)?;
    Some(String::from_utf8_lossy(&output).trim().to_string())
}

fn git_bytes(path: &Path, args: &[&str]) -> Option<Vec<u8>> {
    let output = Command::new("git")
        .arg("-C")
        .arg(git_command_dir(path))
        .args(args)
        .output()
        .ok()?;
    output.status.success().then_some(output.stdout)
}

fn git_command_dir(path: &Path) -> &Path {
    if path.is_file() {
        path.parent().unwrap_or(path)
    } else {
        path
    }
}

#[derive(Debug, PartialEq, Eq)]
struct StatusEntry {
    status: String,
    path: String,
    old_path: Option<String>,
}

fn status_entries(status: &[u8]) -> Vec<StatusEntry> {
    let mut parts = status
        .split(|byte| *byte == 0)
        .filter(|part| !part.is_empty());
    let mut entries = Vec::new();

    while let Some(raw) = parts.next() {
        if raw.len() < 4 {
            continue;
        }

        let status = String::from_utf8_lossy(&raw[..2]).to_string();
        let path = String::from_utf8_lossy(&raw[3..]).replace('\\', "/");
        let old_path = if raw[0] == b'R' || raw[0] == b'C' || raw[1] == b'R' || raw[1] == b'C' {
            parts
                .next()
                .map(|path| String::from_utf8_lossy(path).replace('\\', "/"))
        } else {
            None
        };

        entries.push(StatusEntry {
            status,
            path,
            old_path,
        });
    }

    entries
}

fn is_cache_path(path: &str) -> bool {
    path == CACHE_DIR
        || path.starts_with(&format!("{CACHE_DIR}/"))
        || path.ends_with(&format!("/{CACHE_DIR}"))
        || path.contains(&format!("/{CACHE_DIR}/"))
}

fn hex(bytes: &[u8]) -> String {
    bytes.iter().map(|byte| format!("{byte:02x}")).collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn session_owns_path_configuration_and_profile() {
        let temp = tempdir().expect("tempdir");
        let path = temp.path().to_path_buf();
        let mut repo_config = RepoPilotConfig::default();
        repo_config.architecture.max_file_lines = 777;
        let scan_config = ScanConfig {
            large_file_loc_threshold: 777,
            ..Default::default()
        };

        let session = AnalysisSession::new(
            path.clone(),
            repo_config,
            scan_config,
            FindingVisibilityProfile::Strict,
        );

        assert_eq!(session.analysis_path(), path.as_path());
        assert_eq!(
            session.repo_config().architecture.max_file_lines,
            session.scan_config().large_file_loc_threshold
        );
        assert_eq!(
            session.visibility_profile(),
            FindingVisibilityProfile::Strict
        );
        assert!(!session.revision().id().is_empty());
    }

    #[test]
    fn git_revision_is_stable_for_an_unchanged_workspace() {
        let temp = initialized_git_repo();
        let first = default_session(temp.path());
        let second = default_session(temp.path());

        assert_eq!(first.workspace_root(), second.workspace_root());
        assert_eq!(first.revision(), second.revision());
    }

    #[test]
    fn git_revision_changes_when_a_tracked_file_changes() {
        let temp = initialized_git_repo();
        let first = default_session(temp.path());

        std::fs::write(temp.path().join("src.txt"), "changed\n").expect("update tracked file");
        let second = default_session(temp.path());

        assert_ne!(first.revision(), second.revision());
    }

    #[test]
    fn cache_files_do_not_change_the_workspace_revision() {
        let temp = initialized_git_repo();
        let first = default_session(temp.path());

        let cache = temp.path().join(".repopilot/cache");
        std::fs::create_dir_all(&cache).expect("create cache directory");
        std::fs::write(cache.join("entry.json"), "{}").expect("write cache entry");
        let second = default_session(temp.path());

        assert_eq!(first.revision(), second.revision());
    }

    #[test]
    fn non_git_revision_tracks_content_but_ignores_cache_files() {
        let temp = tempdir().expect("tempdir");
        let source = temp.path().join("source.rs");
        std::fs::write(&source, "pub fn before() {}\n").expect("source");
        let first = WorkspaceRevision::capture(temp.path());

        let cache = temp.path().join(CACHE_DIR);
        std::fs::create_dir_all(&cache).expect("cache dir");
        std::fs::write(cache.join("entry.json"), "{}").expect("cache entry");
        let after_cache = WorkspaceRevision::capture(temp.path());
        assert_eq!(first, after_cache);

        std::fs::write(&source, "pub fn after() {}\n").expect("edit source");
        let after_edit = WorkspaceRevision::capture(temp.path());
        assert_ne!(first, after_edit);
    }

    fn default_session(path: &Path) -> AnalysisSession {
        AnalysisSession::new(
            path.to_path_buf(),
            RepoPilotConfig::default(),
            ScanConfig::default(),
            FindingVisibilityProfile::Default,
        )
    }

    fn initialized_git_repo() -> tempfile::TempDir {
        let temp = tempdir().expect("tempdir");
        git(temp.path(), &["init", "--quiet"]);
        git(
            temp.path(),
            &["config", "user.email", "repopilot@example.test"],
        );
        git(temp.path(), &["config", "user.name", "RepoPilot Test"]);
        std::fs::write(temp.path().join("src.txt"), "initial\n").expect("write tracked file");
        git(temp.path(), &["add", "src.txt"]);
        git(temp.path(), &["commit", "--quiet", "-m", "initial"]);
        temp
    }

    fn git(path: &Path, args: &[&str]) {
        let status = Command::new("git")
            .arg("-C")
            .arg(path)
            .args(args)
            .status()
            .expect("run git");
        assert!(status.success(), "git command failed: {args:?}");
    }
}
