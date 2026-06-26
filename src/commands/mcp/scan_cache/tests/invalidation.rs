use super::*;

#[test]
fn key_is_stable_for_an_unchanged_tree() {
    let (_dir, root) = init_repo();
    let first = cache_key(&root, &args(&root));
    let second = cache_key(&root, &args(&root));
    assert!(first.is_some(), "a git repo is cacheable");
    assert_eq!(first, second, "an unchanged tree yields a stable key");
}

#[test]
fn non_git_path_is_never_cached() {
    let dir = tempfile::tempdir().unwrap();
    assert!(
        cache_key(dir.path(), &args(dir.path())).is_none(),
        "a non-git path must not be cached"
    );
}

#[test]
fn committed_staged_unstaged_and_untracked_changes_invalidate_the_key() {
    let (_dir, root) = init_repo();
    let before = cache_key(&root, &args(&root)).unwrap();

    std::fs::write(
        root.join("src/lib.rs"),
        "pub fn add(a: i32, b: i32) -> i32 { a - b }\n",
    )
    .unwrap();
    let unstaged = cache_key(&root, &args(&root)).unwrap();
    assert_ne!(before, unstaged, "an unstaged edit must invalidate");

    git(&root, &["add", "src/lib.rs"]);
    let staged = cache_key(&root, &args(&root)).unwrap();
    assert_ne!(before, staged, "a staged edit must invalidate");

    git(&root, &["commit", "-qm", "change"]);
    let committed = cache_key(&root, &args(&root)).unwrap();
    assert_ne!(before, committed, "a commit must invalidate");

    std::fs::write(root.join("src/new.rs"), "pub fn extra() {}\n").unwrap();
    let untracked = cache_key(&root, &args(&root)).unwrap();
    assert_ne!(committed, untracked, "an untracked file must invalidate");
}

#[test]
fn changes_outside_subdir_scan_still_invalidate_the_key() {
    let (_dir, root) = init_repo();
    std::fs::write(root.join("README.md"), "initial\n").unwrap();
    git(&root, &["add", "README.md"]);
    git(&root, &["commit", "-qm", "add readme"]);

    let subdir = root.join("src");
    let before = cache_key(&subdir, &args(&subdir)).unwrap();
    std::fs::write(root.join("README.md"), "changed\n").unwrap();
    let after = cache_key(&subdir, &args(&subdir)).unwrap();

    assert_ne!(
        before, after,
        "the cache key must fingerprint the whole Git working tree"
    );
}

#[test]
fn staged_rename_and_deletion_invalidate_the_key() {
    let (_dir, root) = init_repo();
    let before_rename = cache_key(&root, &args(&root)).unwrap();
    git(&root, &["mv", "src/lib.rs", "src/renamed.rs"]);
    let after_rename = cache_key(&root, &args(&root)).unwrap();
    assert_ne!(
        before_rename, after_rename,
        "a staged rename must invalidate"
    );

    git(&root, &["commit", "-qm", "rename"]);
    let before_delete = cache_key(&root, &args(&root)).unwrap();
    git(&root, &["rm", "src/renamed.rs"]);
    let after_delete = cache_key(&root, &args(&root)).unwrap();
    assert_ne!(
        before_delete, after_delete,
        "a staged deletion must invalidate"
    );
}

#[test]
fn profile_and_filter_changes_invalidate_the_key() {
    let (_dir, root) = init_repo();
    let base = cache_key(&root, &args(&root)).unwrap();

    let strict = cache_key(
        &root,
        &json!({ "path": root.to_str().unwrap(), "profile": "strict" }),
    )
    .unwrap();
    assert_ne!(base, strict, "a profile change must invalidate");

    let filtered = cache_key(
        &root,
        &json!({ "path": root.to_str().unwrap(), "filters": { "min_severity": "high" } }),
    )
    .unwrap();
    assert_ne!(base, filtered, "a filter change must invalidate");
}

#[test]
fn explicit_and_discovered_config_content_changes_invalidate_the_key() {
    let (_dir, root) = init_repo();
    let cfg_dir = tempfile::tempdir().unwrap();
    let cfg = cfg_dir.path().join("repopilot.toml");
    let explicit = json!({ "path": root.to_str().unwrap(), "config": cfg.to_str().unwrap() });

    std::fs::write(&cfg, "[architecture]\nmax_file_lines = 300\n").unwrap();
    let explicit_before = cache_key(&root, &explicit).unwrap();
    std::fs::write(&cfg, "[architecture]\nmax_file_lines = 10\n").unwrap();
    let explicit_after = cache_key(&root, &explicit).unwrap();
    assert_ne!(
        explicit_before, explicit_after,
        "an explicit config edit must invalidate"
    );

    std::fs::write(root.join(".gitignore"), "repopilot.toml\n").unwrap();
    git(&root, &["add", ".gitignore"]);
    git(&root, &["commit", "-qm", "ignore config"]);
    let discovered = root.join("repopilot.toml");
    std::fs::write(&discovered, "[architecture]\nmax_file_lines = 300\n").unwrap();
    let root_before = cache_key(&root, &args(&root)).unwrap();
    let subdir_before = cache_key(&root.join("src"), &args(&root.join("src"))).unwrap();

    std::fs::write(&discovered, "[architecture]\nmax_file_lines = 10\n").unwrap();
    let root_after = cache_key(&root, &args(&root)).unwrap();
    let subdir_after = cache_key(&root.join("src"), &args(&root.join("src"))).unwrap();
    assert_ne!(root_before, root_after);
    assert_ne!(subdir_before, subdir_after);
}

#[test]
fn feedback_and_repopilotignore_content_changes_invalidate_the_key() {
    let (_dir, root) = init_repo();
    std::fs::write(
        root.join(".gitignore"),
        ".repopilot/feedback.yml\n.repopilotignore\n",
    )
    .unwrap();
    git(&root, &["add", ".gitignore"]);
    git(&root, &["commit", "-qm", "ignore controls"]);

    let feedback = root.join(".repopilot/feedback.yml");
    std::fs::create_dir_all(feedback.parent().unwrap()).unwrap();
    std::fs::write(
        &feedback,
        "suppressions:\n  - rule_id: security.secret-candidate\n    path: src/lib.rs\n",
    )
    .unwrap();
    let before_feedback = cache_key(&root, &args(&root)).unwrap();
    std::fs::write(
        &feedback,
        "suppressions:\n  - rule_id: architecture.large-file\n    path: src/lib.rs\n",
    )
    .unwrap();
    let after_feedback = cache_key(&root, &args(&root)).unwrap();
    assert_ne!(before_feedback, after_feedback);

    let ignore = root.join(".repopilotignore");
    std::fs::write(&ignore, "target/\n").unwrap();
    let before_ignore = cache_key(&root, &args(&root)).unwrap();
    std::fs::write(&ignore, "target/\ndist/\n").unwrap();
    let after_ignore = cache_key(&root, &args(&root)).unwrap();
    assert_ne!(before_ignore, after_ignore);
}

#[test]
fn parent_gitignore_change_invalidates_subdir_scan_and_never_serves_stale_cache() {
    let (_dir, root) = init_repo();
    std::fs::write(root.join("src/generated.rs"), "pub fn generated() {}\n").unwrap();
    git(&root, &["add", "."]);
    git(&root, &["commit", "-qm", "add generated"]);

    let subdir = root.join("src");
    let arguments = args(&subdir);
    let before = cache_key(&subdir, &arguments).unwrap();
    store(
        &subdir,
        &before,
        &cached_scan_probe("stale-parent-gitignore"),
    );

    std::fs::write(root.join(".gitignore"), "/src/generated.rs\n").unwrap();
    let after = cache_key(&subdir, &arguments).unwrap();
    assert_ne!(
        before, after,
        "a parent .gitignore edit must invalidate a subdir scan"
    );

    let fresh = super::super::super::scan::call(&arguments).unwrap();
    assert!(
        !fresh.contains("stale-parent-gitignore"),
        "a parent .gitignore edit must not serve the stale cache"
    );
}

#[test]
fn git_info_exclude_change_invalidates_subdir_scan() {
    let (_dir, root) = init_repo();
    std::fs::write(root.join("src/generated.rs"), "pub fn generated() {}\n").unwrap();
    git(&root, &["add", "."]);
    git(&root, &["commit", "-qm", "add generated"]);

    let subdir = root.join("src");
    let arguments = args(&subdir);
    let before = cache_key(&subdir, &arguments).unwrap();
    let exclude = git::git_path(&root, "info/exclude").expect("git exclude path");
    std::fs::write(exclude, "/src/generated.rs\n").unwrap();
    let after = cache_key(&subdir, &arguments).unwrap();

    assert_ne!(
        before, after,
        ".git/info/exclude must be part of the cache key"
    );
}

#[test]
fn configured_global_ignore_change_invalidates_subdir_scan() {
    let (_dir, root) = init_repo();
    let global_dir = tempfile::tempdir().unwrap();
    let global_ignore = global_dir.path().join("ignore");
    std::fs::write(&global_ignore, "target/\n").unwrap();
    git(
        &root,
        &[
            "config",
            "core.excludesFile",
            global_ignore.to_str().unwrap(),
        ],
    );

    let subdir = root.join("src");
    let arguments = args(&subdir);
    let before = cache_key(&subdir, &arguments).unwrap();
    std::fs::write(&global_ignore, "target/\nsrc/generated.rs\n").unwrap();
    let after = cache_key(&subdir, &arguments).unwrap();

    assert_ne!(
        before, after,
        "the effective global Git ignore file must be part of the cache key"
    );
}

#[test]
fn ignored_parent_dotignore_change_invalidates_subdir_scan() {
    let (_dir, root) = init_repo();
    std::fs::write(root.join(".gitignore"), ".ignore\n").unwrap();
    git(&root, &["add", ".gitignore"]);
    git(&root, &["commit", "-qm", "ignore dotignore"]);

    let subdir = root.join("src");
    let arguments = args(&subdir);
    std::fs::write(root.join(".ignore"), "dist/\n").unwrap();
    let before = cache_key(&subdir, &arguments).unwrap();
    std::fs::write(root.join(".ignore"), "dist/\nsrc/generated.rs\n").unwrap();
    let after = cache_key(&subdir, &arguments).unwrap();

    assert_ne!(
        before, after,
        "a parent .ignore edit must invalidate even when git status ignores it"
    );
}

#[test]
fn schema_and_package_version_changes_invalidate_the_key() {
    let (_dir, root) = init_repo();
    let arguments = args(&root);

    let base = cache_key_with_metadata(&root, &arguments, "schema-a", "1.0.0").unwrap();
    let schema = cache_key_with_metadata(&root, &arguments, "schema-b", "1.0.0").unwrap();
    let version = cache_key_with_metadata(&root, &arguments, "schema-a", "1.0.1").unwrap();

    assert_ne!(base, schema, "a cache schema change must invalidate");
    assert_ne!(base, version, "a package version change must invalidate");
}
