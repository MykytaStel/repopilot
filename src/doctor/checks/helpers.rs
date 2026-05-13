use std::fs;
use std::path::{Path, PathBuf};

pub fn find_upward(start: &Path, name: &str) -> Option<PathBuf> {
    let mut current = if start.is_file() {
        start.parent()?.to_path_buf()
    } else {
        start.to_path_buf()
    };

    loop {
        let candidate = current.join(name);

        if candidate.exists() {
            return Some(candidate);
        }

        if !current.pop() {
            return None;
        }
    }
}

pub fn has_github_workflows(workflows_dir: &Path) -> bool {
    let Ok(entries) = fs::read_dir(workflows_dir) else {
        return false;
    };

    entries.filter_map(Result::ok).any(|entry| {
        let path = entry.path();

        path.is_file()
            && path
                .extension()
                .and_then(|extension| extension.to_str())
                .is_some_and(|extension| matches!(extension, "yml" | "yaml"))
    })
}

pub fn detect_package_managers(root: &Path) -> Vec<String> {
    let mut managers = Vec::new();

    if root.join("Cargo.toml").is_file() {
        managers.push("Cargo".to_string());
    }

    if root.join("package.json").is_file() {
        if root.join("pnpm-lock.yaml").is_file() {
            managers.push("pnpm".to_string());
        } else if root.join("yarn.lock").is_file() {
            managers.push("Yarn".to_string());
        } else if root.join("bun.lockb").is_file() || root.join("bun.lock").is_file() {
            managers.push("Bun".to_string());
        } else if root.join("package-lock.json").is_file() {
            managers.push("npm".to_string());
        } else {
            managers.push("Node package.json".to_string());
        }
    }

    if root.join("pyproject.toml").is_file() {
        managers.push("Python pyproject".to_string());
    } else if root.join("requirements.txt").is_file() {
        managers.push("pip requirements".to_string());
    }

    if root.join("go.mod").is_file() {
        managers.push("Go modules".to_string());
    }

    if root.join("Gemfile").is_file() {
        managers.push("Bundler".to_string());
    }

    if root.join("composer.json").is_file() {
        managers.push("Composer".to_string());
    }

    if root.join("pom.xml").is_file()
        || root.join("build.gradle").is_file()
        || root.join("build.gradle.kts").is_file()
    {
        managers.push("JVM build".to_string());
    }

    if root.join("Package.swift").is_file() {
        managers.push("Swift Package Manager".to_string());
    }

    managers
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::tempdir;

    #[test]
    fn detects_pnpm_package_manager() {
        let dir = tempdir().expect("tempdir should be created");
        fs::write(dir.path().join("package.json"), "{}").expect("package.json should be written");
        fs::write(dir.path().join("pnpm-lock.yaml"), "lockfileVersion: 9")
            .expect("pnpm lockfile should be written");

        let managers = detect_package_managers(dir.path());

        assert_eq!(managers, vec!["pnpm"]);
    }
}
