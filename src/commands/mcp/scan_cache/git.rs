use sha2::{Digest, Sha256};
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

const CACHE_DIR: &str = ".repopilot/cache";

pub(super) fn working_tree_fingerprint(repo_root: &Path) -> Option<String> {
    let head = git(repo_root, &["rev-parse", "HEAD"])?;
    let status_args = ["status", "--porcelain=v1", "-z", "-uall"];
    let status = git_bytes(repo_root, &status_args)?;
    let mut entries = status_entries(&status);
    entries.sort_by(|left, right| {
        (&left.path, &left.status, &left.old_path).cmp(&(
            &right.path,
            &right.status,
            &right.old_path,
        ))
    });

    let mut hasher = Sha256::new();
    hasher.update(b"head:");
    hasher.update(head.as_bytes());
    hasher.update(b"\nstatus:\n");
    for entry in entries {
        if is_cache_path(&entry.path) {
            continue;
        }
        hasher.update(b"status:");
        hasher.update(entry.status.as_bytes());
        hasher.update(b" ");
        hasher.update(entry.path.as_bytes());
        if let Some(old_path) = &entry.old_path {
            hasher.update(b" <- ");
            hasher.update(old_path.as_bytes());
        }
        hasher.update(b"\n");
        if let Ok(bytes) = fs::read(repo_root.join(&entry.path)) {
            hasher.update(b"file:");
            hasher.update(entry.path.as_bytes());
            hasher.update(b"\n");
            hasher.update(&bytes);
        }
    }
    Some(super::hex(&hasher.finalize()))
}

pub(super) fn git(path: &Path, args: &[&str]) -> Option<String> {
    let output = git_output(path, args)?;
    Some(String::from_utf8_lossy(&output).trim().to_string())
}

pub(super) fn git_root(path: &Path) -> Option<PathBuf> {
    let root = git(path, &["rev-parse", "--show-toplevel"]).map(PathBuf::from)?;
    fs::canonicalize(&root).ok().or(Some(root))
}

pub(super) fn git_dir(path: &Path) -> Option<PathBuf> {
    let raw = git(path, &["rev-parse", "--git-dir"])?;
    resolve_git_path(path, &raw)
}

pub(super) fn git_common_dir(path: &Path) -> Option<PathBuf> {
    let raw = git(path, &["rev-parse", "--git-common-dir"])?;
    resolve_git_path(path, &raw)
}

pub(super) fn git_path(path: &Path, repo_relative: &str) -> Option<PathBuf> {
    let repo_root = git_root(path)?;
    let raw = git(&repo_root, &["rev-parse", "--git-path", repo_relative])?;
    Some(if Path::new(&raw).is_absolute() {
        PathBuf::from(raw)
    } else {
        repo_root.join(raw)
    })
}

pub(super) fn resolve_commit(repo_root: &Path, reference: &str) -> Option<String> {
    let spec = format!("{reference}^{{commit}}");
    git(
        repo_root,
        &[
            "rev-parse",
            "--verify",
            "--quiet",
            "--end-of-options",
            spec.as_str(),
        ],
    )
}

fn git_bytes(path: &Path, args: &[&str]) -> Option<Vec<u8>> {
    git_output(path, args)
}

fn git_output(path: &Path, args: &[&str]) -> Option<Vec<u8>> {
    let output = Command::new("git")
        .arg("-C")
        .arg(git_command_dir(path))
        .args(args)
        .output()
        .ok()?;
    output.status.success().then_some(output.stdout)
}

fn resolve_git_path(path: &Path, raw: &str) -> Option<PathBuf> {
    let raw = PathBuf::from(raw);
    Some(if raw.is_absolute() {
        raw
    } else {
        git_command_dir(path).join(raw)
    })
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

fn is_cache_path(rel: &str) -> bool {
    rel == CACHE_DIR
        || rel.starts_with(&format!("{CACHE_DIR}/"))
        || rel.ends_with(&format!("/{CACHE_DIR}"))
        || rel.contains(&format!("/{CACHE_DIR}/"))
}
