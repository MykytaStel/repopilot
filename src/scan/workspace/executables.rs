use super::detect_workspace_packages;
use std::path::{Path, PathBuf};

/// A package root and whether it declares an executable entrypoint (npm
/// `package.json#bin`, or a Cargo crate with a binary target — `[[bin]]`, a
/// `src/bin/` directory, or `src/main.rs`).
#[derive(Debug, Clone)]
pub struct PackageRoot {
    pub root: PathBuf,
    pub declares_executable: bool,
}

/// All package roots under `root`: the scan root itself when it carries a
/// manifest, plus every detected workspace member. Each is tagged with whether
/// it declares an executable.
///
/// The full set (not just the executable ones) is returned so a file can be
/// resolved to its *nearest* package — see [`path_in_executable_package`]. This
/// is what prevents a CLI at the monorepo root from marking a non-CLI sibling
/// package as executable.
pub fn package_roots(root: &Path) -> Vec<PackageRoot> {
    let mut roots = Vec::new();
    if has_manifest(root) {
        roots.push(PackageRoot {
            root: root.to_path_buf(),
            declares_executable: declares_executable(root),
        });
    }
    for pkg in detect_workspace_packages(root) {
        roots.push(PackageRoot {
            declares_executable: declares_executable(&pkg.root),
            root: pkg.root,
        });
    }
    roots
}

/// Whether `path`'s *nearest* (longest-prefix) package root declares an
/// executable. A file under `packages/web/` resolves against
/// `packages/web/package.json`, not the monorepo root, so a root-level CLI does
/// not downgrade exits in non-CLI sibling packages.
pub fn path_in_executable_package(path: &Path, roots: &[PackageRoot]) -> bool {
    roots
        .iter()
        .filter(|candidate| path.starts_with(&candidate.root))
        .max_by_key(|candidate| candidate.root.components().count())
        .is_some_and(|nearest| nearest.declares_executable)
}

fn has_manifest(dir: &Path) -> bool {
    dir.join("package.json").is_file() || dir.join("Cargo.toml").is_file()
}

fn declares_executable(dir: &Path) -> bool {
    npm_declares_bin(dir) || cargo_declares_bin(dir)
}

/// npm `bin` may be a string (`"./cli.js"`) or an object mapping command names
/// to paths (`{ "tool": "./cli.js" }`); either form declares an executable.
fn npm_declares_bin(dir: &Path) -> bool {
    let Ok(content) = std::fs::read_to_string(dir.join("package.json")) else {
        return false;
    };
    let Ok(value) = serde_json::from_str::<serde_json::Value>(&content) else {
        return false;
    };
    matches!(value.get("bin"), Some(v) if v.is_string() || v.is_object())
}

fn cargo_declares_bin(dir: &Path) -> bool {
    if dir.join("src/bin").is_dir() || dir.join("src/main.rs").is_file() {
        return true;
    }
    std::fs::read_to_string(dir.join("Cargo.toml"))
        .map(|content| content.contains("[[bin]]"))
        .unwrap_or(false)
}

#[cfg(test)]
mod tests {
    use super::{package_roots, path_in_executable_package};
    use std::fs;
    use tempfile::TempDir;

    #[test]
    fn npm_bin_object_marks_package() {
        let dir = TempDir::new().unwrap();
        fs::write(
            dir.path().join("package.json"),
            r#"{ "name": "ignite-cli", "bin": { "ignite": "bin/ignite" } }"#,
        )
        .unwrap();

        let roots = package_roots(dir.path());
        assert!(path_in_executable_package(
            &dir.path().join("src/commands/new.ts"),
            &roots
        ));
    }

    #[test]
    fn npm_bin_string_marks_package() {
        let dir = TempDir::new().unwrap();
        fs::write(
            dir.path().join("package.json"),
            r#"{ "name": "tool", "bin": "./cli.js" }"#,
        )
        .unwrap();

        let roots = package_roots(dir.path());
        assert!(path_in_executable_package(
            &dir.path().join("src/index.js"),
            &roots
        ));
    }

    #[test]
    fn package_without_bin_is_not_executable() {
        let dir = TempDir::new().unwrap();
        fs::write(
            dir.path().join("package.json"),
            r#"{ "name": "web-app", "dependencies": {} }"#,
        )
        .unwrap();

        let roots = package_roots(dir.path());
        assert!(!path_in_executable_package(
            &dir.path().join("src/server.ts"),
            &roots
        ));
    }

    #[test]
    fn cargo_bin_target_marks_package() {
        let dir = TempDir::new().unwrap();
        fs::write(
            dir.path().join("Cargo.toml"),
            "[package]\nname = \"tool\"\n\n[[bin]]\nname = \"tool\"\npath = \"src/main.rs\"\n",
        )
        .unwrap();

        let roots = package_roots(dir.path());
        assert!(path_in_executable_package(
            &dir.path().join("src/lib.rs"),
            &roots
        ));
    }

    #[test]
    fn root_cli_does_not_mark_non_cli_workspace_member() {
        // The exact boundary the path-prefix check must respect: a CLI declared
        // at the monorepo root must not make a non-CLI sibling package executable.
        let dir = TempDir::new().unwrap();
        fs::write(
            dir.path().join("package.json"),
            r#"{ "name": "monorepo", "workspaces": ["packages/*"], "bin": { "my-tool": "./src/cli.ts" } }"#,
        )
        .unwrap();

        let web = dir.path().join("packages/web");
        let core = dir.path().join("packages/core");
        fs::create_dir_all(&web).unwrap();
        fs::create_dir_all(&core).unwrap();
        fs::write(web.join("package.json"), r#"{ "name": "web" }"#).unwrap();
        fs::write(
            core.join("package.json"),
            r#"{ "name": "core", "bin": "./index.js" }"#,
        )
        .unwrap();

        let roots = package_roots(dir.path());

        // A file directly under the root belongs to the root CLI package.
        assert!(path_in_executable_package(
            &dir.path().join("src/cli.ts"),
            &roots
        ));
        // A file in the non-CLI `web` package resolves to `packages/web` (no bin),
        // NOT the root, so it is not executable.
        assert!(!path_in_executable_package(
            &web.join("src/server.ts"),
            &roots
        ));
        // A file in the CLI `core` package is executable on its own merit.
        assert!(path_in_executable_package(
            &core.join("src/main.js"),
            &roots
        ));
    }
}
