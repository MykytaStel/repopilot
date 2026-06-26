use serde_json::Value;
use std::cmp::Ordering;
use std::fs;
use std::path::{Component, Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering as AtomicOrdering};
use std::time::SystemTime;

pub(super) const MAX_VALID_ENTRIES: usize = 32;
const CACHE_GIT_PATH: &str = "repopilot/cache/mcp-scan";

static TEMP_COUNTER: AtomicU64 = AtomicU64::new(0);

pub(super) fn load(path: &Path, key: &str) -> Option<String> {
    let cached = fs::read_to_string(cache_file(path, key)?).ok()?;
    valid_cached_scan(&cached).then_some(cached)
}

pub(super) fn store(path: &Path, key: &str, output: &str) {
    let Some(dir) = cache_dir(path) else {
        return;
    };
    if fs::create_dir_all(&dir).is_err() {
        return;
    }

    let file = dir.join(format!("{key}.json"));
    let tmp = unique_tmp_file(&dir, key);
    if fs::write(&tmp, output).is_err() {
        let _ = fs::remove_file(&tmp);
        return;
    }
    if fs::rename(&tmp, &file).is_ok() {
        prune(&dir);
    } else {
        let _ = fs::remove_file(&tmp);
    }
}

pub(super) fn cache_dir(path: &Path) -> Option<PathBuf> {
    let candidate = normalize_path(super::git::git_path(path, CACHE_GIT_PATH)?);
    let git_dir = canonical_metadata_dir(super::git::git_dir(path)?)?;
    let common_dir = canonical_metadata_dir(super::git::git_common_dir(path)?)?;

    (candidate.starts_with(&git_dir) || candidate.starts_with(&common_dir)).then_some(candidate)
}

fn cache_file(path: &Path, key: &str) -> Option<PathBuf> {
    Some(cache_dir(path)?.join(format!("{key}.json")))
}

fn unique_tmp_file(dir: &Path, key: &str) -> PathBuf {
    let suffix = TEMP_COUNTER.fetch_add(1, AtomicOrdering::Relaxed);
    dir.join(format!(
        "{}.{}.{}.json.tmp",
        key,
        std::process::id(),
        suffix
    ))
}

fn prune(dir: &Path) {
    let mut entries = valid_entries(dir);
    if entries.len() <= MAX_VALID_ENTRIES {
        return;
    }
    entries.sort_by(|left, right| match left.modified.cmp(&right.modified) {
        Ordering::Equal => left.path.cmp(&right.path),
        order => order,
    });

    let remove_count = entries.len().saturating_sub(MAX_VALID_ENTRIES);
    for entry in entries.into_iter().take(remove_count) {
        let _ = fs::remove_file(entry.path);
    }
}

fn valid_entries(dir: &Path) -> Vec<CacheEntry> {
    let Ok(entries) = fs::read_dir(dir) else {
        return Vec::new();
    };
    entries
        .filter_map(Result::ok)
        .filter_map(|entry| cache_entry(entry.path()))
        .collect()
}

fn cache_entry(path: PathBuf) -> Option<CacheEntry> {
    if path.extension().and_then(|ext| ext.to_str()) != Some("json") {
        return None;
    }
    let text = fs::read_to_string(&path).ok()?;
    if !valid_cached_scan(&text) {
        return None;
    }
    let modified = fs::metadata(&path)
        .and_then(|metadata| metadata.modified())
        .unwrap_or(SystemTime::UNIX_EPOCH);
    Some(CacheEntry { path, modified })
}

struct CacheEntry {
    path: PathBuf,
    modified: SystemTime,
}

fn valid_cached_scan(text: &str) -> bool {
    let Ok(value) = serde_json::from_str::<Value>(text) else {
        return false;
    };
    value
        .get("schema_version")
        .and_then(Value::as_str)
        .is_some()
        && value
            .get("report")
            .and_then(|report| report.get("kind"))
            .and_then(Value::as_str)
            == Some("scan")
}

fn canonical_metadata_dir(path: PathBuf) -> Option<PathBuf> {
    fs::canonicalize(path).ok().map(normalize_path)
}

fn normalize_path(path: PathBuf) -> PathBuf {
    let mut normalized = PathBuf::new();
    for component in path.components() {
        match component {
            Component::Prefix(prefix) => normalized.push(prefix.as_os_str()),
            Component::RootDir => normalized.push(component.as_os_str()),
            Component::CurDir => {}
            Component::ParentDir => {
                normalized.pop();
            }
            Component::Normal(part) => normalized.push(part),
        }
    }
    normalized
}
