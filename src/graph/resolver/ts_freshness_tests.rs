use super::*;
use std::fs;

fn config(root: &Path, name: &str, target: &str) {
    fs::write(
        root.join(name),
        format!(r#"{{"compilerOptions":{{"paths":{{"@api":["{target}"]}}}}}}"#),
    )
    .unwrap();
}

#[test]
fn effective_alias_inputs_are_fresh_in_one_process() {
    let temp = tempfile::tempdir().unwrap();
    let root = temp.path();
    let files = [root.join("one.ts"), root.join("two.ts")]
        .into_iter()
        .collect();
    let resolve = || resolve_ts("@api", &root.join("consumer.ts"), root, &files);
    assert_eq!(resolve(), None);
    config(root, "jsconfig.json", "one");
    assert_eq!(resolve(), Some(root.join("one.ts")));
    config(root, "tsconfig.json", "two");
    assert_eq!(resolve(), Some(root.join("two.ts")));
    config(root, "jsconfig.json", "two");
    assert_eq!(resolve(), Some(root.join("two.ts")));
    config(root, "tsconfig.json", "one");
    assert_eq!(resolve(), Some(root.join("one.ts")));
    fs::remove_file(root.join("tsconfig.json")).unwrap();
    assert_eq!(resolve(), Some(root.join("two.ts")));
    fs::remove_file(root.join("jsconfig.json")).unwrap();
    assert_eq!(resolve(), None);
}

#[test]
fn alias_resolution_in_fresh_process() {
    let Ok(root) = std::env::var("REPOPILOT_ALIAS_TEST_ROOT") else {
        return;
    };
    let root = Path::new(&root);
    let files = [root.join("one.ts"), root.join("two.ts")]
        .into_iter()
        .collect();
    assert_eq!(
        resolve_ts("@api", &root.join("consumer.ts"), root, &files),
        Some(root.join("two.ts"))
    );
}

#[test]
fn unchanged_effective_input_matches_fresh_process() {
    let temp = tempfile::tempdir().unwrap();
    let root = temp.path();
    config(root, "tsconfig.json", "two");
    let first = tsconfig_paths(root);
    let second = tsconfig_paths(root);
    assert_eq!(first[0].roots, second[0].roots);
    let status = std::process::Command::new(std::env::current_exe().unwrap())
        .args([
            "--exact",
            "graph::resolver::ts::freshness_tests::alias_resolution_in_fresh_process",
        ])
        .env("REPOPILOT_ALIAS_TEST_ROOT", root)
        .status()
        .unwrap();
    assert!(status.success());
}

#[test]
fn readable_invalid_tsconfig_still_wins_over_jsconfig() {
    let temp = tempfile::tempdir().unwrap();
    let root = temp.path();
    config(root, "jsconfig.json", "one");
    fs::write(root.join("tsconfig.json"), "invalid").unwrap();
    assert!(tsconfig_paths(root).is_empty());
    fs::remove_file(root.join("tsconfig.json")).unwrap();
    assert_eq!(tsconfig_paths(root)[0].roots, vec![root.join("one")]);
}
