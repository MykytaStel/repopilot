use super::{resolve_jvm, type_path_candidates};
use std::collections::HashSet;
use std::path::PathBuf;

fn files(paths: &[&str]) -> HashSet<PathBuf> {
    paths.iter().map(PathBuf::from).collect()
}

#[test]
fn a_single_module_maven_layout_still_resolves() {
    // The layout the fixed source-root list used to handle must keep working.
    let known =
        files(&["/repo/src/main/java/org/springframework/samples/petclinic/owner/Owner.java"]);
    assert_eq!(
        resolve_jvm(
            "org.springframework.samples.petclinic.owner.Owner",
            &known,
            &["java"]
        ),
        Some(PathBuf::from(
            "/repo/src/main/java/org/springframework/samples/petclinic/owner/Owner.java"
        ))
    );
}

#[test]
fn a_gradle_module_source_root_resolves() {
    // Catches the regression this replaces: a root-relative probe misses every
    // `<module>/src/main/kotlin` in a multi-module build.
    let known = files(&[
        "/repo/core/data/src/main/kotlin/com/google/samples/apps/nia/core/data/Repo.kt",
        "/repo/feature/foryou/impl/src/main/kotlin/com/google/samples/apps/nia/feature/foryou/ForYouViewModel.kt",
    ]);
    assert_eq!(
        resolve_jvm(
            "com.google.samples.apps.nia.core.data.Repo",
            &known,
            &["kt", "java"]
        ),
        Some(PathBuf::from(
            "/repo/core/data/src/main/kotlin/com/google/samples/apps/nia/core/data/Repo.kt"
        ))
    );
}

#[test]
fn a_companion_member_import_resolves_to_its_declaring_type() {
    let known = files(&["/repo/lint/src/main/kotlin/com/example/lint/DesignSystemDetector.kt"]);
    assert_eq!(
        resolve_jvm(
            "com.example.lint.DesignSystemDetector.Companion.ISSUE",
            &known,
            &["kt", "java"]
        ),
        Some(PathBuf::from(
            "/repo/lint/src/main/kotlin/com/example/lint/DesignSystemDetector.kt"
        ))
    );
}

#[test]
fn a_top_level_declaration_in_an_unrelated_file_stays_unresolved() {
    // Kotlin top-level functions do not have to live in a file named after
    // them, so no path convention can find `followableTopicTestData`. Guessing
    // would invent an edge.
    let known = files(&["/repo/core/testing/src/main/kotlin/com/example/testing/data/TestData.kt"]);
    assert_eq!(
        resolve_jvm(
            "com.example.testing.data.followableTopicTestData",
            &known,
            &["kt", "java"]
        ),
        None
    );
}

#[test]
fn production_source_sets_win_over_test_source_sets() {
    let known = files(&[
        "/repo/app/src/main/kotlin/com/example/app/Config.kt",
        "/repo/app/src/test/kotlin/com/example/app/Config.kt",
        "/repo/app/src/androidTest/kotlin/com/example/app/Config.kt",
    ]);
    assert_eq!(
        resolve_jvm("com.example.app.Config", &known, &["kt", "java"]),
        Some(PathBuf::from(
            "/repo/app/src/main/kotlin/com/example/app/Config.kt"
        ))
    );
}

#[test]
fn two_production_variants_of_one_type_stay_unresolved() {
    // Gradle product flavors can declare the same type twice; which one the
    // build picks is a variant decision this resolver cannot make, so the
    // import is reported unresolved instead of wired arbitrarily.
    let known = files(&[
        "/repo/core/analytics/src/demo/kotlin/com/example/analytics/AnalyticsModule.kt",
        "/repo/core/analytics/src/prod/kotlin/com/example/analytics/AnalyticsModule.kt",
    ]);
    assert_eq!(
        resolve_jvm("com.example.analytics.AnalyticsModule", &known, &["kt"]),
        None
    );
}

#[test]
fn a_package_path_must_start_at_a_directory_boundary() {
    // `.../mycom/example/Foo.kt` must not satisfy an import of `com.example.Foo`.
    let known = files(&["/repo/src/main/kotlin/mycom/example/Foo.kt"]);
    assert_eq!(resolve_jvm("com.example.Foo", &known, &["kt"]), None);
}

#[test]
fn star_imports_and_bare_names_resolve_nothing() {
    let known = files(&["/repo/src/main/kotlin/com/example/Foo.kt"]);
    assert_eq!(resolve_jvm("com.example.*", &known, &["kt"]), None);
    assert_eq!(resolve_jvm("Foo", &known, &["kt"]), None);
}

#[test]
fn member_stripping_stops_before_it_reaches_a_package() {
    // Catches dropping so many segments that a package directory name is
    // treated as a type and matched against an unrelated file.
    assert_eq!(
        type_path_candidates("com.example.Detector.Companion.ISSUE"),
        vec![
            "com/example/Detector/Companion/ISSUE",
            "com/example/Detector/Companion",
            "com/example/Detector",
        ]
    );
    assert_eq!(
        type_path_candidates("com.example.data.helper"),
        vec!["com/example/data/helper"],
        "a lowercase tail is not a type, so nothing is stripped"
    );
}
