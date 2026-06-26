use super::*;

#[test]
fn unchanged_resolved_changed_base_produces_a_cache_hit() {
    let (_dir, root) = init_repo();
    git(&root, &["branch", "scan-base", "HEAD"]);
    std::fs::write(root.join("src/lib.rs"), "pub fn changed() -> i32 { 2 }\n").unwrap();
    git(&root, &["commit", "-am", "head change", "-q"]);

    let arguments = changed_args(&root, "scan-base");
    let first = cache_key(&root, &arguments).expect("resolved base is cacheable");
    let second = cache_key(&root, &arguments).expect("resolved base remains cacheable");
    assert_eq!(first, second);

    let cached = cached_scan_probe("changed-base-hit");
    store(&root, &first, &cached);
    assert_eq!(
        super::super::super::scan::call(&arguments).unwrap(),
        cached,
        "unchanged named base should serve the cached report"
    );
}

#[test]
fn moving_the_same_named_changed_base_invalidates_the_key() {
    let (_dir, root) = init_repo();
    git(&root, &["branch", "scan-base", "HEAD"]);
    std::fs::write(root.join("src/lib.rs"), "pub fn changed() -> i32 { 2 }\n").unwrap();
    git(&root, &["commit", "-am", "head change", "-q"]);

    let arguments = changed_args(&root, "scan-base");
    let before = cache_key(&root, &arguments).expect("initial base is cacheable");

    git(&root, &["branch", "-f", "scan-base", "HEAD"]);
    let after = cache_key(&root, &arguments).expect("moved base is cacheable");

    assert_ne!(
        before, after,
        "moving a named base ref to a new commit must invalidate"
    );
}

#[test]
fn unresolvable_changed_base_disables_disk_cache() {
    let (_dir, root) = init_repo();
    git(&root, &["branch", "scan-base", "HEAD"]);
    std::fs::write(root.join("src/lib.rs"), "pub fn changed() -> i32 { 2 }\n").unwrap();
    git(&root, &["commit", "-am", "head change", "-q"]);

    let good_args = changed_args(&root, "scan-base");
    let good_key = cache_key(&root, &good_args).expect("valid base is cacheable");
    store(&root, &good_key, &cached_scan_probe("must-not-serve"));

    let bad_args = changed_args(&root, "missing-base");
    assert!(
        cache_key(&root, &bad_args).is_none(),
        "an unresolvable base ref must disable disk caching"
    );
    let outcome = super::super::super::scan::call(&bad_args);
    assert!(
        outcome.is_err(),
        "the fresh scan should surface the bad ref"
    );
    assert!(
        !format!("{outcome:?}").contains("must-not-serve"),
        "an unresolvable base must not serve an existing cache entry"
    );
}
