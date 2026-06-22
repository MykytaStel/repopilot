use super::detect_workspace_packages;
use std::path::{Path, PathBuf};

/// Directories that are the root of a package declaring an executable
/// entrypoint — an npm `package.json#bin`, or a Cargo crate with a binary
/// target (`[[bin]]`, a `src/bin/` directory, or `src/main.rs`). Every source
/// file under such a directory belongs to a CLI tool, where `process.exit`-style
/// host termination is an intended boundary rather than a hazard in reusable
/// code.
///
/// Considers the scan root itself plus any detected workspace members, so a
/// monorepo's CLI package is recognized while its non-CLI siblings (e.g. a web
/// app under `apps/web` with a `domain/commands/` directory) are not.
pub fn cli_executable_roots(root: &Path) -> Vec<PathBuf> {
    let mut candidates: Vec<PathBuf> = vec![root.to_path_buf()];
    candidates.extend(detect_workspace_packages(root).into_iter().map(|p| p.root));

    candidates
        .into_iter()
        .filter(|dir| declares_executable(dir))
        .collect()
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
    use super::cli_executable_roots;
    use std::fs;
    use tempfile::TempDir;

    #[test]
    fn npm_bin_object_marks_root_as_cli() {
        let dir = TempDir::new().unwrap();
        fs::write(
            dir.path().join("package.json"),
            r#"{ "name": "ignite-cli", "bin": { "ignite": "bin/ignite" } }"#,
        )
        .unwrap();

        let roots = cli_executable_roots(dir.path());
        assert_eq!(roots, vec![dir.path().to_path_buf()]);
    }

    #[test]
    fn npm_bin_string_marks_root_as_cli() {
        let dir = TempDir::new().unwrap();
        fs::write(
            dir.path().join("package.json"),
            r#"{ "name": "tool", "bin": "./cli.js" }"#,
        )
        .unwrap();

        assert_eq!(cli_executable_roots(dir.path()).len(), 1);
    }

    #[test]
    fn package_without_bin_is_not_cli() {
        let dir = TempDir::new().unwrap();
        fs::write(
            dir.path().join("package.json"),
            r#"{ "name": "web-app", "dependencies": {} }"#,
        )
        .unwrap();

        assert!(cli_executable_roots(dir.path()).is_empty());
    }

    #[test]
    fn cargo_bin_target_marks_root_as_cli() {
        let dir = TempDir::new().unwrap();
        fs::write(
            dir.path().join("Cargo.toml"),
            "[package]\nname = \"tool\"\n\n[[bin]]\nname = \"tool\"\npath = \"src/main.rs\"\n",
        )
        .unwrap();

        assert_eq!(cli_executable_roots(dir.path()).len(), 1);
    }

    #[test]
    fn only_cli_workspace_member_is_marked() {
        let dir = TempDir::new().unwrap();
        fs::write(
            dir.path().join("package.json"),
            r#"{ "name": "monorepo", "workspaces": ["packages/*"] }"#,
        )
        .unwrap();

        let cli = dir.path().join("packages/cli");
        let web = dir.path().join("packages/web");
        fs::create_dir_all(&cli).unwrap();
        fs::create_dir_all(&web).unwrap();
        fs::write(
            cli.join("package.json"),
            r#"{ "name": "cli", "bin": "./index.js" }"#,
        )
        .unwrap();
        fs::write(web.join("package.json"), r#"{ "name": "web" }"#).unwrap();

        let roots = cli_executable_roots(dir.path());
        assert!(roots.contains(&cli));
        assert!(!roots.contains(&web));
    }
}
