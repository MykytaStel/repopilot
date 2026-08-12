use super::*;

#[test]
fn records_and_counts_unresolved_imports_per_source() {
    let mut stats = ImportResolutionStats::default();
    assert!(stats.is_empty());

    stats.record(Path::new("src/a.ts"), "./missing");
    stats.record(Path::new("src/a.ts"), "../gone/helper");
    stats.record(Path::new("src/b.ts"), "./missing");

    assert!(!stats.is_empty());
    assert_eq!(stats.total(), 3);
    assert_eq!(stats.unresolved_internal_by_source.len(), 2);
}

#[test]
fn typed_evidence_preserves_definitive_candidates_and_limited_imports() {
    let root = Path::new("/repo");
    let mut stats = ImportResolutionStats::default();
    stats.record_classified(Path::new("/repo/src/app.ts"), "./missing.ts", root);
    stats.record_classified(Path::new("/repo/src/app.ts"), "./generated-client", root);

    let evidence = stats.evidence().collect::<Vec<_>>();
    assert_eq!(evidence.len(), 2);
    assert_eq!(evidence[0].raw_import, "./generated-client");
    assert_eq!(evidence[0].kind, UnresolvedImportKind::RelativePath);
    assert_eq!(
        evidence[0].proof,
        UnresolvedImportProof::Limited(UnresolvedImportLimitation::AmbiguousTarget)
    );
    assert_eq!(evidence[1].raw_import, "./missing.ts");
    assert!(matches!(
        &evidence[1].proof,
        UnresolvedImportProof::DefinitiveLocalCandidates(candidates)
            if candidates == &vec![
                PathBuf::from("/repo/src/missing.ts"),
                PathBuf::from("/repo/src/missing.tsx"),
                PathBuf::from("/repo/src/missing.js"),
                PathBuf::from("/repo/src/missing.jsx"),
            ]
    ));
}

#[test]
fn could_target_stem_matches_path_and_dotted_module_segments() {
    let mut stats = ImportResolutionStats::default();
    stats.record(Path::new("src/a.ts"), "../legacy/Utils.js");
    stats.record(Path::new("apps/web/main.py"), "app.services.foo");

    assert!(stats.could_target_stem("utils"));
    assert!(stats.could_target_stem("foo"));
    assert!(stats.could_target_stem("app"));
    assert!(!stats.could_target_stem("services"));
    assert!(!stats.could_target_stem(""));
}

#[test]
fn relative_import_detection_matches_dot_prefixes_only() {
    assert!(is_relative_import("./a"));
    assert!(is_relative_import("../a"));
    assert!(is_relative_import(".python_module"));
    assert!(!is_relative_import("react"));
    assert!(!is_relative_import("@scope/pkg"));
}

#[test]
fn internal_import_classifier_separates_workspace_from_third_party() {
    let repo_dirs: HashSet<String> = ["app", "components", "ml"]
        .into_iter()
        .map(String::from)
        .collect();

    assert!(is_unresolved_internal_import("./helper", &repo_dirs));
    assert!(is_unresolved_internal_import(
        "@/components/Button",
        &repo_dirs
    ));
    assert!(is_unresolved_internal_import("~/lib/util", &repo_dirs));
    assert!(is_unresolved_internal_import("app.ml.train", &repo_dirs));
    assert!(is_unresolved_internal_import(
        "components/Button",
        &repo_dirs
    ));
    assert!(!is_unresolved_internal_import("react", &repo_dirs));
    assert!(!is_unresolved_internal_import("@angular/core", &repo_dirs));
    assert!(!is_unresolved_internal_import("numpy", &repo_dirs));
    assert!(!is_unresolved_internal_import(
        "django.db.models",
        &repo_dirs
    ));
}

#[test]
fn repo_directory_names_collects_parent_segments_only() {
    let paths = [
        Path::new("apps/ml/app/train.py"),
        Path::new("apps/web/src/index.ts"),
    ];
    let dirs = repo_directory_names(paths);

    for expected in ["apps", "ml", "app", "web", "src"] {
        assert!(dirs.contains(expected));
    }
    assert!(!dirs.contains("train"));
    assert!(!dirs.contains("index"));
}
