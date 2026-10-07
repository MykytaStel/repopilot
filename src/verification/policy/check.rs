use crate::verification::VerificationRole;
use globset::GlobSet;
use std::path::{Path, PathBuf};

#[derive(Debug)]
pub struct ValidatedCheck {
    pub(super) id: String,
    pub(crate) role: VerificationRole,
    pub(crate) program: ValidatedProgram,
    pub(crate) resolved_program: Option<PathBuf>,
    pub(crate) executable_sha256: Option<String>,
    pub(crate) args: Vec<String>,
    pub(crate) working_directory: PathBuf,
    pub(crate) working_directory_label: String,
    pub(crate) timeout_seconds: u64,
    pub(crate) max_output_bytes: usize,
    pub(crate) paths: Option<GlobSet>,
    pub(crate) path_patterns: Vec<String>,
    pub(super) cache_enabled: bool,
}

#[derive(Debug, PartialEq, Eq)]
pub(crate) enum ValidatedProgram {
    Bare(String),
    RepositoryRelative(PathBuf),
}

pub(crate) fn resolve_bare_program(program: &str, working_directory: &Path) -> Option<PathBuf> {
    let path = std::env::var_os("PATH")?;
    let extensions = std::env::var("PATHEXT").ok();
    let candidates = program_candidates(program, extensions.as_deref(), cfg!(windows));
    std::env::split_paths(&path)
        .flat_map(|directory| {
            let directory = if directory.is_absolute() {
                directory
            } else {
                working_directory.join(directory)
            };
            candidates.iter().map(move |name| directory.join(name))
        })
        .find(|candidate| candidate.is_file() && is_executable(candidate))
}

fn program_candidates(program: &str, path_ext: Option<&str>, windows: bool) -> Vec<String> {
    let mut candidates = vec![program.to_string()];
    if windows && Path::new(program).extension().is_none() {
        let extensions = path_ext.unwrap_or(".COM;.EXE;.BAT;.CMD");
        candidates.extend(
            extensions
                .split(';')
                .filter(|extension| !extension.is_empty())
                .map(|extension| format!("{program}{extension}")),
        );
    }
    candidates
}

#[cfg(unix)]
fn is_executable(path: &Path) -> bool {
    use std::os::unix::fs::PermissionsExt;

    std::fs::metadata(path).is_ok_and(|metadata| metadata.permissions().mode() & 0o111 != 0)
}

#[cfg(not(unix))]
fn is_executable(path: &Path) -> bool {
    path.is_file()
}

impl ValidatedCheck {
    pub fn id(&self) -> &str {
        &self.id
    }

    pub fn is_applicable<'a>(&self, paths: impl IntoIterator<Item = &'a Path>) -> bool {
        let Some(patterns) = &self.paths else {
            return true;
        };
        paths.into_iter().any(|path| {
            let normalized = path.to_string_lossy().replace('\\', "/");
            patterns.is_match(normalized)
        })
    }

    pub fn cache_enabled(&self) -> bool {
        self.cache_enabled
    }

    pub fn has_stable_executable_identity(&self) -> bool {
        self.resolved_program.is_some() && self.executable_sha256.is_some()
    }

    /// Confirms the executable still has the content identity captured at selection.
    pub fn executable_identity_matches(&self) -> bool {
        let (Some(program), Some(expected)) = (
            self.resolved_program.as_deref(),
            self.executable_sha256.as_ref(),
        ) else {
            return false;
        };
        super::super::file_hash::sha256_file_hex(program).as_ref() == Some(expected)
    }

    /// Safe details shown to a user before the MCP client requests approval.
    /// This intentionally excludes environment values and process output.
    pub fn approval_details(&self) -> String {
        let program = self
            .resolved_program
            .as_ref()
            .map(|program| program.display().to_string())
            .unwrap_or_else(|| match &self.program {
                ValidatedProgram::Bare(program) => program.clone(),
                ValidatedProgram::RepositoryRelative(program) => program.display().to_string(),
            });
        let args = serde_json::to_string(&self.args).unwrap_or_else(|_| "[]".to_string());
        format!(
            "Check: {}\nProgram: {}\nArguments (exact): {}\nWorking directory: {}\nThis process runs with the host user's permissions and is not sandboxed. Environment values and process output are not shown.",
            self.id,
            program,
            args,
            self.working_directory.display()
        )
    }

    /// Compares every validated input that can change what the check executes.
    pub fn same_execution_policy(&self, other: &Self) -> bool {
        self.id == other.id
            && self.role == other.role
            && self.program == other.program
            && self.resolved_program == other.resolved_program
            && self.executable_sha256 == other.executable_sha256
            && self.args == other.args
            && self.working_directory == other.working_directory
            && self.working_directory_label == other.working_directory_label
            && self.timeout_seconds == other.timeout_seconds
            && self.max_output_bytes == other.max_output_bytes
            && self.path_patterns == other.path_patterns
            && self.cache_enabled == other.cache_enabled
    }
}

#[cfg(test)]
mod tests {
    use super::super::select_checks;
    use crate::config::loader::parse_config;
    use std::path::PathBuf;
    use tempfile::tempdir;

    #[test]
    fn policy_comparison_covers_execution_values_and_prompt_shows_only_approval_details() {
        let root = tempdir().expect("root");
        let original = selected(root.path(), "printf ok", "30");
        let same = selected(root.path(), "printf ok", "30");
        let changed = selected(root.path(), "printf changed", "30");
        let mut changed_resolution = selected(root.path(), "printf ok", "30");
        changed_resolution.resolved_program = Some(PathBuf::from("/different/tool"));

        assert!(original.same_execution_policy(&same));
        assert!(!original.same_execution_policy(&changed));
        assert!(!original.same_execution_policy(&changed_resolution));
        let details = original.approval_details();
        assert!(details.contains("Check: unit"));
        let program = original
            .resolved_program
            .as_ref()
            .map_or("sh".to_string(), |path| path.display().to_string());
        assert!(details.contains(&format!("Program: {program}")));
        assert!(details.contains("Arguments (exact): [\"-c\",\"printf ok\"]"));
        assert!(details.contains("Working directory:"));
        assert!(details.contains("not sandboxed"));
        assert!(!details.contains("SECRET"));
    }

    #[test]
    fn policy_comparison_rejects_executable_content_replacement() {
        let root = tempdir().expect("root");
        let executable = root.path().join("tools/check");
        std::fs::create_dir_all(executable.parent().expect("tools directory"))
            .expect("create tools directory");
        std::fs::write(&executable, "original executable").expect("initial executable");
        let original = selected_program(root.path(), "tools/check");

        std::fs::write(&executable, "replacement executable").expect("replace executable");
        let replacement = selected_program(root.path(), "tools/check");

        assert!(!original.same_execution_policy(&replacement));
    }

    fn selected(root: &std::path::Path, command: &str, timeout: &str) -> super::ValidatedCheck {
        let config = parse_config(
            &format!(
                "[[verification.checks]]\nid = \"unit\"\nrole = \"test\"\nprogram = \"sh\"\nargs = [\"-c\", \"{command}\"]\ntimeout_seconds = {timeout}\n"
            ),
            None,
        )
        .expect("config");
        select_checks(root, &config.verification.checks, &["unit".to_string()])
            .expect("selected check")
            .remove(0)
    }

    fn selected_program(root: &std::path::Path, program: &str) -> super::ValidatedCheck {
        let config = parse_config(
            &format!(
                "[[verification.checks]]\nid = \"unit\"\nrole = \"test\"\nprogram = \"{program}\"\n"
            ),
            None,
        )
        .expect("config");
        select_checks(root, &config.verification.checks, &["unit".to_string()])
            .expect("selected check")
            .remove(0)
    }
}
