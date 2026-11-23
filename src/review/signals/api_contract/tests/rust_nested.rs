use super::*;

#[test]
fn rust_removed_enum_variant_resolves_nested_module_callers() {
    let temp = TempDir::new().unwrap();
    let root = temp.path();
    init_repo(root);
    write(root, "src/api.rs", "pub mod types;\n");
    write(root, "src/api/types.rs", "pub enum State { Old, Ready }\n");
    write(
        root,
        "src/lib.rs",
        "mod api; fn run() { let _ = api::types::State::Old; }\n",
    );
    let exporter = changed("src/api/types.rs", ChangeStatus::Modified);
    let pre = ReviewSource::new(
        "pub enum State { Old, Ready }\n".into(),
        Some("Rust".into()),
    );
    let post = ReviewSource::new("pub enum State { Ready }\n".into(), Some("Rust".into()));
    let graph = coupling_graph(&[
        ("src/lib.rs", "src/api.rs"),
        ("src/api.rs", "src/api/types.rs"),
    ]);
    let signals = detect_removed_export_imports(
        root,
        DiffTarget::WorkingTree,
        &[ChangedReviewSources {
            file: &exporter,
            pre: Some(&pre),
            post: Some(&post),
        }],
        Some(&graph),
    );

    assert_eq!(signals.len(), 1, "transitive module caller must resolve");
    assert_eq!(signals[0].importer_path, PathBuf::from("src/lib.rs"));
    assert_eq!(signals[0].module_specifier, "mod::api::types");
    assert_eq!(signals[0].exported_name, "Old");
}

#[test]
fn v028_rust_ignores_redirected_intermediate_modules() {
    let temp = TempDir::new().unwrap();
    let root = temp.path();
    init_repo(root);
    write(
        root,
        "src/api.rs",
        "#[path = \"alternate.rs\"] pub mod types;\n",
    );
    write(root, "src/alternate.rs", "pub enum State { Old, Ready }\n");
    write(
        root,
        "src/other.rs",
        "#[path = \"api/types.rs\"] pub mod other_types;\n",
    );
    write(root, "src/api/types.rs", "pub enum State { Old, Ready }\n");
    write(
        root,
        "src/lib.rs",
        "mod api; mod other; fn run() { let _ = api::types::State::Old; }\n",
    );
    let exporter = changed("src/api/types.rs", ChangeStatus::Modified);
    let pre = ReviewSource::new(
        "pub enum State { Old, Ready }\n".into(),
        Some("Rust".into()),
    );
    let post = ReviewSource::new("pub enum State { Ready }\n".into(), Some("Rust".into()));
    let graph = coupling_graph(&[
        ("src/lib.rs", "src/api.rs"),
        ("src/api.rs", "src/alternate.rs"),
        ("src/lib.rs", "src/other.rs"),
        ("src/other.rs", "src/api/types.rs"),
    ]);
    let signals = detect_removed_export_imports(
        root,
        DiffTarget::WorkingTree,
        &[ChangedReviewSources {
            file: &exporter,
            pre: Some(&pre),
            post: Some(&post),
        }],
        Some(&graph),
    );

    assert!(signals.is_empty());
}
