use super::*;

#[test]
fn rust_removed_public_function_has_proven_surviving_import() {
    let temp = TempDir::new().unwrap();
    let root = temp.path();
    init_repo(root);
    write(root, "src/api.rs", "pub fn save() {}\n");
    write(
        root,
        "src/lib.rs",
        "mod api;\nuse self::api::load;\nfn run() { load(); }\n",
    );
    let api = changed("src/api.rs", ChangeStatus::Modified);
    let pre = ReviewSource::new("pub fn load() {}\n".into(), Some("Rust".into()));
    let post = ReviewSource::new("pub fn save() {}\n".into(), Some("Rust".into()));
    let signals = detect_removed_export_imports(
        root,
        DiffTarget::WorkingTree,
        &[ChangedReviewSources {
            file: &api,
            pre: Some(&pre),
            post: Some(&post),
        }],
        Some(&coupling_graph(&[("src/lib.rs", "src/api.rs")])),
    );
    assert_eq!(signals.len(), 1);
    assert_eq!(signals[0].exported_name, "load");
    assert_eq!(signals[0].importer_path, PathBuf::from("src/lib.rs"));
}

fn rust_case(
    before: &str,
    after: &str,
    caller: &str,
    ambiguous: bool,
) -> Vec<super::super::RemovedExportSignal> {
    let temp = TempDir::new().unwrap();
    let root = temp.path();
    init_repo(root);
    write(root, "src/api.rs", after);
    write(root, "src/lib.rs", caller);
    if ambiguous {
        write(root, "src/api/mod.rs", after);
    }
    let api = changed("src/api.rs", ChangeStatus::Modified);
    let pre = ReviewSource::new(before.into(), Some("Rust".into()));
    let post = ReviewSource::new(after.into(), Some("Rust".into()));
    detect_removed_export_imports(
        root,
        DiffTarget::WorkingTree,
        &[ChangedReviewSources {
            file: &api,
            pre: Some(&pre),
            post: Some(&post),
        }],
        Some(&coupling_graph(&[("src/lib.rs", "src/api.rs")])),
    )
}

#[test]
fn rust_direct_alias_and_calls_retain_distinct_spans() {
    for caller in [
        "mod api;\nuse api::load as first;\nuse self::api::load as second;\n",
        "mod api;\nfn run() { api::load(); self::api::load(); }\n",
        "mod api;\nuse crate::api::load as first;\nuse crate::api::load as second;\n",
    ] {
        let signals = rust_case("pub fn load() {}\n", "pub fn save() {}\n", caller, false);
        assert_eq!(signals.len(), 2, "{caller}");
        assert_ne!(signals[0].byte_start, signals[1].byte_start);
    }
}

#[test]
fn rust_preserved_and_coordinated_functions_are_safe() {
    for (before, after, caller) in [
        (
            "pub fn load() {}",
            "pub(crate) fn load() {}",
            "mod api; use api::load;",
        ),
        (
            "pub fn load() {}",
            "fn load() {}",
            "mod api; use api::load;",
        ),
        (
            "pub fn load() {}",
            "pub const load: fn() = || {};",
            "mod api; use api::load;",
        ),
        (
            "pub fn load() {}",
            "pub struct load;",
            "mod api; use api::load;",
        ),
        (
            "pub fn load() {}",
            "pub fn load(x: usize) {}",
            "mod api; use api::load;",
        ),
        (
            "pub fn load() {}",
            "pub fn save() {}",
            "mod api; use api::save;",
        ),
        (
            "pub fn load() {}",
            "pub fn save() {}",
            "mod api; fn run() {}",
        ),
        (
            "fn load() {}",
            "pub fn save() {}",
            "mod api; use api::load;",
        ),
        (
            "pub(crate) fn load() {}",
            "pub fn save() {}",
            "mod api; use api::load;",
        ),
        (
            "pub trait T { fn load(); }",
            "pub trait T {}",
            "mod api; use api::load;",
        ),
    ] {
        assert!(
            rust_case(before, after, caller, false).is_empty(),
            "{before} -> {after}"
        );
    }
}

#[test]
fn rust_uncertain_paths_and_opaque_syntax_cannot_prove_removal() {
    let before = "pub fn load() {}";
    let after = "pub fn save() {}";
    for caller in [
        "use crate::api::load;",
        "mod api { pub fn load() {} } use api::load;",
        "#[cfg(feature = \"api\")] mod api; use api::load;",
        "#[path = \"api.rs\"] mod api; use api::load;",
        "mod api; use api::*;",
        "mod api; use api::{load};",
        "mod api; fn run() { object.load(); }",
        "mod api; fn run<api>() { api::load(); }",
        "mod api; fn run() { use other as api; api::load(); }",
        "mod api; fn run() { generated!(); api::load(); }",
        "mod api; pub use api::load;",
        "mod api; use other::load;",
        "mod api; fn run( { api::load(); }",
        "mod api; fn run() { let text = \"api::load()\"; }",
    ] {
        assert!(
            rust_case(before, after, caller, false).is_empty(),
            "{caller}"
        );
    }
    for after in [
        "pub use other::load;",
        "pub(crate) use other::load;",
        "unsafe extern \"C\" { pub fn load(); }",
        "#[cfg(feature = \"api\")] pub fn save() {}",
        "generated!();",
        "fn broken( {",
    ] {
        assert!(rust_case(before, after, "mod api; use api::load;", false).is_empty());
    }
    assert!(rust_case(before, after, "mod api; use api::load;", true).is_empty());
}

#[test]
fn rust_local_type_shadowing_is_not_a_module_call() {
    let caller = "mod api; fn run() { struct api; impl api { fn load() {} } api::load(); }";
    assert!(rust_case("pub fn load() {}", "pub fn save() {}", caller, false).is_empty());
    assert_eq!(
        rust_case(
            "pub fn load() {}",
            "pub fn save() {}",
            "mod api; fn run() { api::load(); }",
            false
        )
        .len(),
        1
    );
}

#[test]
fn rust_shadowing_respects_block_scope_and_explicit_qualification() {
    for item in [
        "struct api;",
        "enum api { Variant }",
        "union api { value: usize }",
        "type api = ();",
        "trait api {}",
    ] {
        let caller = format!("mod api; fn run() {{ api::load(); {item} }}");
        assert!(
            rust_case("pub fn load() {}", "pub fn save() {}", &caller, false).is_empty(),
            "{item}"
        );
    }
    for caller in [
        "mod api; fn run() { struct api; self::api::load(); crate::api::load(); }",
        "mod api; use api::load; fn run() { struct api; api::load(); } fn other() { api::load(); }",
        "mod api; fn run() { { struct api; api::load(); } api::load(); self::api::load(); }",
    ] {
        assert_eq!(
            rust_case("pub fn load() {}", "pub fn save() {}", caller, false).len(),
            2,
            "{caller}"
        );
    }
}
