use sha2::{Digest, Sha256};
use std::fs;
use std::io::Read;
use std::path::{Path, PathBuf};
use std::process::Command;

pub(super) fn working_tree_fingerprint(repo_root: &Path) -> Option<String> {
    let head = git(repo_root, &["rev-parse", "HEAD"])?;
    let status_args = ["status", "--porcelain=v1", "-z", "-uall"];
    let status = git_bytes(repo_root, &status_args)?;
    let mut entries = status_entries(&status)?;
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
    hash_index(&mut hasher, repo_root)?;
    let confinement = repopilot::path_security::RootConfinement::new(repo_root).ok()?;
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
        let path = repo_root.join(&entry.path);
        match fs::symlink_metadata(&path) {
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
                hasher.update(b"missing");
            }
            Ok(metadata) if metadata.is_file() => {
                let relative = Path::new(&entry.path);
                let resolved = confinement.resolve_existing(relative, "dirty input").ok()?;
                hasher.update(b"file:");
                hasher.update(entry.path.as_bytes());
                hasher.update(b"\n");
                hash_content(&mut hasher, &resolved)?;
            }
            // Unknown input identities must disable disk cache reuse.
            _ => return None,
        }
    }
    Some(super::hex(&hasher.finalize()))
}

fn hash_index(hasher: &mut Sha256, root: &Path) -> Option<()> {
    let index = git_bytes(root, &["ls-files", "--stage", "-z"])?;
    hasher.update(b"\nindex:");
    for entry in index
        .split(|byte| *byte == 0)
        .filter(|entry| !entry.is_empty())
    {
        let separator = entry.iter().position(|byte| *byte == b'\t')?;
        let path = std::str::from_utf8(&entry[separator + 1..]).ok()?;
        if !is_cache_path(path) {
            hasher.update((entry.len() as u64).to_le_bytes());
            hasher.update(entry);
        }
    }
    Some(())
}

fn hash_content(hasher: &mut Sha256, path: &Path) -> Option<()> {
    let mut file = fs::File::open(path).ok()?;
    let mut content = Sha256::new();
    let mut buffer = [0_u8; 16 * 1024];
    loop {
        let count = file.read(&mut buffer).ok()?;
        if count == 0 {
            break;
        }
        content.update(&buffer[..count]);
    }
    hasher.update(content.finalize());
    Some(())
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

fn status_entries(status: &[u8]) -> Option<Vec<StatusEntry>> {
    let mut parts = status
        .split(|byte| *byte == 0)
        .filter(|part| !part.is_empty());
    let mut entries = Vec::new();

    while let Some(raw) = parts.next() {
        if raw.len() < 4 || raw[2] != b' ' {
            return None;
        }
        let status = std::str::from_utf8(&raw[..2]).ok()?.to_string();
        let path = std::str::from_utf8(&raw[3..]).ok()?.to_string();
        let old_path = if raw[0] == b'R' || raw[0] == b'C' || raw[1] == b'R' || raw[1] == b'C' {
            Some(std::str::from_utf8(parts.next()?).ok()?.to_string())
        } else {
            None
        };
        entries.push(StatusEntry {
            status,
            path,
            old_path,
        });
    }

    Some(entries)
}

fn is_cache_path(rel: &str) -> bool {
    let normalized = rel.replace('\\', "/");
    let clean = normalized.trim_start_matches("./");
    let components = clean.split('/').collect::<Vec<_>>();
    components
        .windows(2)
        .any(|pair| pair == [".repopilot", "cache"] || pair == [".repopilot", "history"])
}
