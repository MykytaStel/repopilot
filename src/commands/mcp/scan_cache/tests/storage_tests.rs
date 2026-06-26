use super::*;
use serde_json::Value;
use std::sync::Arc;

#[test]
fn cache_files_are_git_owned_not_worktree_and_do_not_dirty_status() {
    let (_dir, root) = init_repo();
    let before_status = git_stdout(&root, &["status", "--porcelain", "--", "."]);
    let key = cache_key(&root, &args(&root)).unwrap();

    store(&root, &key, &cached_scan_probe("git-owned"));

    let cache_dir = cache_dir_for(&root);
    assert!(
        !cache_dir.starts_with(root.join(".repopilot")),
        "cache dir must not be the repository working-tree RepoPilot dir"
    );
    assert!(cache_dir.join(format!("{key}.json")).is_file());
    assert!(!root.join(".repopilot/cache/mcp-scan").exists());
    assert_eq!(
        before_status,
        git_stdout(&root, &["status", "--porcelain", "--", "."]),
        "writing cache files must not alter git status"
    );
}

#[test]
fn root_and_subdir_scans_share_the_same_git_owned_cache_area() {
    let (_dir, root) = init_repo();
    let subdir = root.join("src");

    assert_eq!(cache_dir_for(&root), cache_dir_for(&subdir));

    let arguments = args(&subdir);
    let key = cache_key(&subdir, &arguments).unwrap();
    let cached = cached_scan_probe("subdir-hit");
    store(&subdir, &key, &cached);

    assert_eq!(cache_key(&subdir, &arguments).unwrap(), key);
    assert_eq!(load(&subdir, &key).unwrap(), cached);
    assert!(!subdir.join(".repopilot/cache/mcp-scan").exists());
}

#[test]
fn worktree_git_file_uses_git_metadata_or_disables_cache() {
    let (dir, root) = init_repo();
    let linked = dir.path().join("linked-worktree");
    git(
        &root,
        &[
            "worktree",
            "add",
            "-q",
            "-b",
            "linked-worktree",
            linked.to_str().unwrap(),
            "HEAD",
        ],
    );
    assert!(
        linked.join(".git").is_file(),
        "linked worktree should use a .git file"
    );

    let arguments = args(&linked);
    let Some(key) = cache_key(&linked, &arguments) else {
        return;
    };
    let cache_dir = cache_dir_for(&linked);
    assert!(!cache_dir.starts_with(linked.join(".repopilot")));

    store(&linked, &key, &cached_scan_probe("worktree"));
    assert!(load(&linked, &key).is_some());
}

#[test]
fn corrupt_cache_entry_is_ignored() {
    let (_dir, root) = init_repo();
    let arguments = args(&root);
    let key = cache_key(&root, &arguments).expect("git repo is cacheable");

    store(&root, &key, "{not-json");
    let fresh = super::super::super::scan::call(&arguments).unwrap();

    assert_ne!(
        fresh, "{not-json",
        "corrupt cache content must not be served"
    );
    assert!(fresh.contains("schema_version"));
}

#[test]
fn retention_keeps_at_most_32_valid_scan_entries_and_preserves_unrelated_files() {
    let (_dir, root) = init_repo();
    let cache_dir = cache_dir_for(&root);
    std::fs::create_dir_all(&cache_dir).unwrap();
    let unrelated = cache_dir.join("notes.txt");
    let invalid = cache_dir.join("invalid.json");
    let tmp = cache_dir.join("leftover.json.tmp");
    std::fs::write(&unrelated, "keep me").unwrap();
    std::fs::write(&invalid, "{not-json").unwrap();
    std::fs::write(&tmp, "partial").unwrap();

    for index in 0..40 {
        store(
            &root,
            &format!("{index:064x}"),
            &cached_scan_probe(&format!("entry-{index}")),
        );
    }

    assert!(
        valid_scan_entry_count(&cache_dir) <= storage::MAX_VALID_ENTRIES,
        "retention should cap valid scan entries"
    );
    assert!(unrelated.exists(), "unrelated files must not be pruned");
    assert!(
        invalid.exists(),
        "invalid JSON is ignored, not treated as cache"
    );
    assert!(
        tmp.exists(),
        "temporary-looking unrelated files are preserved"
    );
    assert!(
        load(&root, &format!("{:064x}", 0)).is_none(),
        "oldest valid entries should be pruned first"
    );
    assert!(
        load(&root, &format!("{:064x}", 39)).is_some(),
        "newer valid entries should be retained"
    );
}

#[test]
fn concurrent_writers_use_unique_temp_files_and_leave_valid_json() {
    let (_dir, root) = init_repo();
    let key = cache_key(&root, &args(&root)).unwrap();
    let root = Arc::new(root);

    let mut handles = Vec::new();
    for index in 0..8 {
        let root = Arc::clone(&root);
        let key = key.clone();
        handles.push(std::thread::spawn(move || {
            store(&root, &key, &cached_scan_probe(&format!("writer-{index}")));
        }));
    }
    for handle in handles {
        handle.join().expect("writer thread");
    }

    let cached = load(&root, &key).expect("one writer should win");
    let parsed: Value = serde_json::from_str(&cached).expect("cached report is valid JSON");
    assert_eq!(parsed["report"]["kind"], "scan");
    assert!(
        std::fs::read_dir(cache_dir_for(&root))
            .unwrap()
            .filter_map(Result::ok)
            .all(|entry| entry.path().extension().and_then(|ext| ext.to_str()) != Some("tmp")),
        "unique temp files should not leave partial cache files"
    );
}

fn valid_scan_entry_count(cache_dir: &std::path::Path) -> usize {
    std::fs::read_dir(cache_dir)
        .unwrap()
        .filter_map(Result::ok)
        .filter(|entry| entry.path().extension().and_then(|ext| ext.to_str()) == Some("json"))
        .filter(|entry| {
            std::fs::read_to_string(entry.path())
                .ok()
                .and_then(|text| serde_json::from_str::<Value>(&text).ok())
                .and_then(|value| {
                    value
                        .get("report")?
                        .get("kind")?
                        .as_str()
                        .map(str::to_owned)
                })
                == Some("scan".to_string())
        })
        .count()
}
