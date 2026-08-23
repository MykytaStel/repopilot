use super::{CacheEntry, MAX_VALID_ENTRIES, VerificationCache, cache_file, valid_entry};
use crate::scan::session::WorkspaceRevision;
use crate::verification::cache::key::VerificationCacheKey;
use crate::verification::{VerificationOutcome, VerificationRole, VerificationStatus};
use serde_json::Value;
use std::fs;
use std::sync::Arc;
use std::thread;
use tempfile::tempdir;

fn key(value: usize) -> VerificationCacheKey {
    VerificationCacheKey(format!("{value:064x}"))
}

fn outcome(revision: &WorkspaceRevision) -> VerificationOutcome {
    VerificationOutcome {
        check_id: "unit".into(),
        role: VerificationRole::Test,
        status: VerificationStatus::Passed,
        duration_ms: 25,
        exit_code: Some(0),
        working_directory: ".".into(),
        stdout_excerpt: "token=[REDACTED]".into(),
        stderr_excerpt: String::new(),
        stdout_truncated: false,
        stderr_truncated: false,
        revision_before: revision.id().into(),
        revision_after: revision.id().into(),
        revision_compatible: true,
        limitations: Vec::new(),
        reused: false,
    }
}

#[test]
fn passed_compatible_outcome_round_trips_as_reused_evidence() {
    let temp = tempdir().expect("temp dir");
    let revision = WorkspaceRevision::capture(temp.path());
    let cache = VerificationCache::new(temp.path());
    cache.store(&key(1), &outcome(&revision));

    let stored = fs::read_to_string(cache_file(temp.path(), &key(1))).expect("stored entry");
    let parsed: CacheEntry = serde_json::from_str(&stored).expect("entry must deserialize");
    assert!(valid_entry(&parsed, &key(1), &revision));

    let hit = cache.load(&key(1), &revision).expect("cache hit");

    assert_eq!(hit.status, VerificationStatus::Passed);
    assert!(hit.reused);
    assert!(stored.contains("[REDACTED]"));
    assert!(!stored.contains("original-secret"));
}

#[test]
fn failed_or_revision_incompatible_outcomes_are_not_stored() {
    let temp = tempdir().expect("temp dir");
    let revision = WorkspaceRevision::capture(temp.path());
    let cache = VerificationCache::new(temp.path());
    let mut failed = outcome(&revision);
    failed.status = VerificationStatus::Failed;
    cache.store(&key(1), &failed);
    assert!(!cache_file(temp.path(), &key(1)).exists());

    let mut incompatible = outcome(&revision);
    incompatible.revision_compatible = false;
    incompatible.revision_after = "different".into();
    cache.store(&key(2), &incompatible);
    assert!(!cache_file(temp.path(), &key(2)).exists());
}

#[test]
fn corrupt_or_incompatible_entries_are_cache_misses() {
    let temp = tempdir().expect("temp dir");
    let revision = WorkspaceRevision::capture(temp.path());
    let cache = VerificationCache::new(temp.path());
    cache.store(&key(1), &outcome(&revision));
    let path = cache_file(temp.path(), &key(1));

    fs::write(&path, "not-json").expect("corrupt entry");
    assert!(cache.load(&key(1), &revision).is_none());

    for (field, replacement) in [
        ("schema_version", Value::from(99)),
        ("repopilot_version", Value::from("other")),
        ("key", Value::from("wrong")),
    ] {
        cache.store(&key(1), &outcome(&revision));
        let mut value: Value =
            serde_json::from_str(&fs::read_to_string(&path).expect("read valid entry"))
                .expect("valid entry json");
        value[field] = replacement;
        fs::write(
            &path,
            serde_json::to_vec(&value).expect("serialize mutation"),
        )
        .expect("mutate entry");
        assert!(cache.load(&key(1), &revision).is_none(), "field: {field}");
    }

    cache.store(&key(1), &outcome(&revision));
    fs::write(temp.path().join("tracked.txt"), "changed").expect("workspace edit");
    let changed = WorkspaceRevision::capture(temp.path());
    assert!(cache.load(&key(1), &changed).is_none());
}

#[test]
fn concurrent_writers_leave_one_valid_entry_without_temp_files() {
    let temp = tempdir().expect("temp dir");
    let revision = WorkspaceRevision::capture(temp.path());
    let cache = Arc::new(VerificationCache::new(temp.path()));
    let handles = (0..8)
        .map(|_| {
            let cache = Arc::clone(&cache);
            let revision = revision.clone();
            thread::spawn(move || cache.store(&key(1), &outcome(&revision)))
        })
        .collect::<Vec<_>>();
    for handle in handles {
        handle.join().expect("writer");
    }

    assert!(cache.load(&key(1), &revision).is_some());
    let stored_file = cache_file(temp.path(), &key(1));
    let directory = stored_file.parent().expect("cache parent");
    assert!(
        fs::read_dir(directory)
            .expect("cache entries")
            .filter_map(Result::ok)
            .all(|entry| !entry.file_name().to_string_lossy().ends_with(".tmp"))
    );
}

#[test]
fn retention_is_bounded_and_preserves_other_cache_namespaces() {
    let temp = tempdir().expect("temp dir");
    let revision = WorkspaceRevision::capture(temp.path());
    let cache = VerificationCache::new(temp.path());
    let unrelated = temp.path().join(".repopilot/cache/file_hashes.json");
    fs::create_dir_all(unrelated.parent().expect("cache root")).expect("cache root");
    fs::write(&unrelated, "keep").expect("unrelated cache");

    for value in 0..=MAX_VALID_ENTRIES {
        cache.store(&key(value), &outcome(&revision));
    }

    let stored_file = cache_file(temp.path(), &key(0));
    let directory = stored_file.parent().expect("parent");
    let json_count = fs::read_dir(directory)
        .expect("entries")
        .filter_map(Result::ok)
        .filter(|entry| entry.path().extension().and_then(|ext| ext.to_str()) == Some("json"))
        .count();
    assert_eq!(json_count, MAX_VALID_ENTRIES);
    assert_eq!(fs::read_to_string(unrelated).expect("unrelated"), "keep");
}

#[test]
fn storage_failures_are_misses_and_do_not_change_workspace_revision() {
    let temp = tempdir().expect("temp dir");
    let before = WorkspaceRevision::capture(temp.path());
    let cache = VerificationCache::new(temp.path());
    cache.store(&key(1), &outcome(&before));
    assert_eq!(WorkspaceRevision::capture(temp.path()), before);

    let blocked = tempdir().expect("blocked temp dir");
    fs::write(blocked.path().join(".repopilot"), "not a directory").expect("blocking file");
    let blocked_revision = WorkspaceRevision::capture(blocked.path());
    let blocked_cache = VerificationCache::new(blocked.path());
    blocked_cache.store(&key(2), &outcome(&blocked_revision));
    assert!(blocked_cache.load(&key(2), &blocked_revision).is_none());
}
