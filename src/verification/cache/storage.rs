use super::key::{CACHE_SCHEMA_VERSION, VerificationCacheKey};
use crate::scan::session::WorkspaceRevision;
use crate::verification::{VerificationOutcome, VerificationStatus};
use serde::{Deserialize, Serialize};
use std::cmp::Ordering;
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering as AtomicOrdering};
use std::time::SystemTime;

pub(super) const MAX_VALID_ENTRIES: usize = 64;
const CACHE_DIRECTORY: &str = ".repopilot/cache/verification/v1";
static TEMP_COUNTER: AtomicU64 = AtomicU64::new(0);

#[derive(Debug, Clone)]
pub(crate) struct VerificationCache {
    root: PathBuf,
}

#[derive(Deserialize, Serialize)]
struct CacheEntry {
    schema_version: u32,
    repopilot_version: String,
    key: String,
    outcome: VerificationOutcome,
}

impl VerificationCache {
    pub(crate) fn new(root: &Path) -> Self {
        Self {
            root: root.to_path_buf(),
        }
    }

    pub(crate) fn load(
        &self,
        key: &VerificationCacheKey,
        revision: &WorkspaceRevision,
    ) -> Option<VerificationOutcome> {
        let text = fs::read_to_string(cache_file(&self.root, key)).ok()?;
        let mut entry: CacheEntry = serde_json::from_str(&text).ok()?;
        if !valid_entry(&entry, key, revision) {
            return None;
        }
        entry.outcome.reused = true;
        Some(entry.outcome)
    }

    pub(crate) fn store(&self, key: &VerificationCacheKey, outcome: &VerificationOutcome) {
        if !reusable_outcome(outcome) {
            return;
        }
        let directory = cache_directory(&self.root);
        if fs::create_dir_all(&directory).is_err() {
            return;
        }
        let mut stored_outcome = outcome.clone();
        stored_outcome.reused = false;
        let entry = CacheEntry {
            schema_version: CACHE_SCHEMA_VERSION,
            repopilot_version: env!("CARGO_PKG_VERSION").to_string(),
            key: key.as_str().to_string(),
            outcome: stored_outcome,
        };
        let Ok(bytes) = serde_json::to_vec(&entry) else {
            return;
        };
        let temporary = temporary_file(&directory, key);
        if fs::write(&temporary, bytes).is_err() {
            let _ = fs::remove_file(&temporary);
            return;
        }
        if fs::rename(&temporary, cache_file(&self.root, key)).is_ok() {
            prune(&directory);
        } else {
            let _ = fs::remove_file(&temporary);
        }
    }
}

fn valid_entry(
    entry: &CacheEntry,
    key: &VerificationCacheKey,
    revision: &WorkspaceRevision,
) -> bool {
    entry.schema_version == CACHE_SCHEMA_VERSION
        && entry.repopilot_version == env!("CARGO_PKG_VERSION")
        && entry.key == key.as_str()
        && reusable_outcome(&entry.outcome)
        && entry.outcome.revision_before == revision.id()
        && entry.outcome.revision_after == revision.id()
}

fn reusable_outcome(outcome: &VerificationOutcome) -> bool {
    outcome.status == VerificationStatus::Passed
        && outcome.revision_compatible
        && outcome.revision_before == outcome.revision_after
}

fn cache_directory(root: &Path) -> PathBuf {
    root.join(CACHE_DIRECTORY)
}

fn cache_file(root: &Path, key: &VerificationCacheKey) -> PathBuf {
    cache_directory(root).join(format!("{}.json", key.as_str()))
}

fn temporary_file(directory: &Path, key: &VerificationCacheKey) -> PathBuf {
    let counter = TEMP_COUNTER.fetch_add(1, AtomicOrdering::Relaxed);
    directory.join(format!(
        "{}.{}.{}.json.tmp",
        key.as_str(),
        std::process::id(),
        counter
    ))
}

fn prune(directory: &Path) {
    let mut entries = valid_cache_files(directory);
    if entries.len() <= MAX_VALID_ENTRIES {
        return;
    }
    entries.sort_by(|left, right| match left.modified.cmp(&right.modified) {
        Ordering::Equal => left.path.cmp(&right.path),
        ordering => ordering,
    });
    let remove_count = entries.len() - MAX_VALID_ENTRIES;
    for entry in entries.into_iter().take(remove_count) {
        let _ = fs::remove_file(entry.path);
    }
}

fn valid_cache_files(directory: &Path) -> Vec<CacheFile> {
    let Ok(entries) = fs::read_dir(directory) else {
        return Vec::new();
    };
    entries
        .filter_map(Result::ok)
        .filter_map(|entry| {
            let path = entry.path();
            let text = fs::read_to_string(&path).ok()?;
            serde_json::from_str::<CacheEntry>(&text).ok()?;
            let modified = entry
                .metadata()
                .and_then(|metadata| metadata.modified())
                .unwrap_or(SystemTime::UNIX_EPOCH);
            Some(CacheFile { path, modified })
        })
        .collect()
}

struct CacheFile {
    path: PathBuf,
    modified: SystemTime,
}

#[cfg(test)]
mod tests;
