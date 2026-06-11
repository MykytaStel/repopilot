use super::detect_workspace_packages;
use std::fs;
use std::path::Path;
use tempfile::TempDir;

fn write(root: &Path, relative: &str, content: &str) {
    let path = root.join(relative);
    fs::create_dir_all(path.parent().unwrap()).unwrap();
    fs::write(path, content).unwrap();
}

fn names(root: &Path) -> Vec<String> {
    let mut names: Vec<String> = detect_workspace_packages(root)
        .into_iter()
        .map(|pkg| pkg.name)
        .collect();
    names.sort();
    names
}

#[test]
fn detects_npm_workspace_globs() {
    let dir = TempDir::new().unwrap();
    let root = dir.path();
    write(root, "package.json", r#"{ "workspaces": ["packages/*"] }"#);
    write(
        root,
        "packages/web/package.json",
        r#"{ "name": "@acme/web" }"#,
    );
    write(
        root,
        "packages/api/package.json",
        r#"{ "name": "@acme/api" }"#,
    );

    assert_eq!(names(root), vec!["@acme/api", "@acme/web"]);
}

#[test]
fn detects_cargo_workspace_members() {
    let dir = TempDir::new().unwrap();
    let root = dir.path();
    write(
        root,
        "Cargo.toml",
        "[workspace]\nmembers = [\"crates/core\", \"crates/cli\"]\n",
    );
    write(
        root,
        "crates/core/Cargo.toml",
        "[package]\nname = \"core\"\n",
    );
    write(root, "crates/cli/Cargo.toml", "[package]\nname = \"cli\"\n");

    assert_eq!(names(root), vec!["cli", "core"]);
}

#[test]
fn detects_go_work_use_directories() {
    let dir = TempDir::new().unwrap();
    let root = dir.path();
    write(
        root,
        "go.work",
        "go 1.22\n\nuse (\n\t./svc-a\n\t./svc-b\n)\n",
    );
    write(
        root,
        "svc-a/go.mod",
        "module example.com/svc-a\n\ngo 1.22\n",
    );
    write(
        root,
        "svc-b/go.mod",
        "module example.com/svc-b\n\ngo 1.22\n",
    );

    assert_eq!(names(root), vec!["example.com/svc-a", "example.com/svc-b"]);
}

#[test]
fn detects_single_line_go_work_use() {
    let dir = TempDir::new().unwrap();
    let root = dir.path();
    write(root, "go.work", "go 1.22\n\nuse ./service\n");
    write(root, "service/go.mod", "module example.com/service\n");

    assert_eq!(names(root), vec!["example.com/service"]);
}

#[test]
fn go_work_use_without_go_mod_is_skipped() {
    let dir = TempDir::new().unwrap();
    let root = dir.path();
    write(root, "go.work", "use ./not-a-module\n");
    fs::create_dir_all(root.join("not-a-module")).unwrap();

    assert!(detect_workspace_packages(root).is_empty());
}

#[test]
fn non_workspace_root_has_no_packages() {
    let dir = TempDir::new().unwrap();
    let root = dir.path();
    write(root, "package.json", r#"{ "name": "solo" }"#);

    assert!(detect_workspace_packages(root).is_empty());
}
