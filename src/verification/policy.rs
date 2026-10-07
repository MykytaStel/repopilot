use super::file_hash::sha256_file_hex;
use crate::config::model::VerificationCheckConfig;
use crate::review::diff::OwnedDiffTarget;
use globset::{Glob, GlobSet, GlobSetBuilder};
use std::collections::{BTreeMap, BTreeSet};
use std::fmt;
use std::path::{Component, Path, PathBuf};
use std::process::Command;

mod check;
pub use check::ValidatedCheck;
pub(crate) use check::ValidatedProgram;

const MAX_TIMEOUT_SECONDS: u64 = 1_800;
const MAX_OUTPUT_BYTES: usize = 1_048_576;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct VerificationPolicyError(String);

impl fmt::Display for VerificationPolicyError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(&self.0)
    }
}

impl std::error::Error for VerificationPolicyError {}

pub fn select_checks(
    root: &Path,
    configured: &[VerificationCheckConfig],
    selected: &[String],
) -> Result<Vec<ValidatedCheck>, VerificationPolicyError> {
    let root = root.canonicalize().map_err(|error| {
        VerificationPolicyError(format!("failed to resolve repository root: {error}"))
    })?;
    let mut checks = BTreeMap::new();
    for config in configured {
        if checks.contains_key(&config.id) {
            return Err(VerificationPolicyError(format!(
                "duplicate verification check id `{}`",
                config.id
            )));
        }
        checks.insert(config.id.clone(), validate_check(&root, config)?);
    }

    let selected = selected.iter().collect::<BTreeSet<_>>();
    let mut result = Vec::with_capacity(selected.len());
    for id in selected {
        let check = checks.remove(id).ok_or_else(|| {
            VerificationPolicyError(format!("unknown verification check id `{id}`"))
        })?;
        result.push(check);
    }
    Ok(result)
}

pub fn validate_review_target(
    root: &Path,
    target: &OwnedDiffTarget,
    ignored_paths: &[String],
) -> Result<(), VerificationPolicyError> {
    let OwnedDiffTarget::Refs { head, .. } = target else {
        return Ok(());
    };
    let selected = git_text(root, &["rev-parse", "--verify", head])?;
    let checkout = git_text(root, &["rev-parse", "--verify", "HEAD"])?;
    if selected != checkout {
        return Err(VerificationPolicyError(format!(
            "verification head `{head}` does not match the current checkout HEAD"
        )));
    }
    let status = Command::new("git")
        .arg("-C")
        .arg(root)
        .args(["status", "--porcelain=v1", "-z", "-uall"])
        .output()
        .map_err(|error| {
            VerificationPolicyError(format!("failed to inspect checkout state: {error}"))
        })?;
    if !status.status.success() {
        return Err(VerificationPolicyError(
            "failed to inspect checkout state".to_string(),
        ));
    }
    let ignored = compile_ignored_paths(ignored_paths)?;
    let dirty = status
        .stdout
        .split(|byte| *byte == 0)
        .filter(|entry| !entry.is_empty())
        .any(|entry| {
            let path = String::from_utf8_lossy(entry.get(3..).unwrap_or_default());
            path != ".repopilot/cache"
                && !path.starts_with(".repopilot/cache/")
                && !ignored.is_match(path.as_ref())
        });
    if dirty {
        return Err(VerificationPolicyError(
            "ref-range verification requires a clean current checkout".to_string(),
        ));
    }
    Ok(())
}

fn compile_ignored_paths(paths: &[String]) -> Result<GlobSet, VerificationPolicyError> {
    let mut builder = GlobSetBuilder::new();
    for path in paths {
        let normalized = path.trim_matches('/');
        for pattern in [normalized.to_string(), format!("{normalized}/**")] {
            builder.add(Glob::new(&pattern).map_err(|error| {
                VerificationPolicyError(format!("invalid configured ignore `{path}`: {error}"))
            })?);
        }
    }
    builder
        .build()
        .map_err(|error| VerificationPolicyError(format!("invalid configured ignores: {error}")))
}

fn git_text(root: &Path, args: &[&str]) -> Result<String, VerificationPolicyError> {
    let output = Command::new("git")
        .arg("-C")
        .arg(root)
        .args(args)
        .output()
        .map_err(|error| VerificationPolicyError(format!("failed to run git: {error}")))?;
    if !output.status.success() {
        return Err(VerificationPolicyError(format!(
            "git could not resolve verification target: {}",
            String::from_utf8_lossy(&output.stderr).trim()
        )));
    }
    Ok(String::from_utf8_lossy(&output.stdout).trim().to_string())
}

fn validate_check(
    root: &Path,
    config: &VerificationCheckConfig,
) -> Result<ValidatedCheck, VerificationPolicyError> {
    validate_id(&config.id)?;
    if !(1..=MAX_TIMEOUT_SECONDS).contains(&config.timeout_seconds) {
        return Err(VerificationPolicyError(format!(
            "verification check `{}` timeout_seconds must be between 1 and {MAX_TIMEOUT_SECONDS}",
            config.id
        )));
    }
    if !(1..=MAX_OUTPUT_BYTES).contains(&config.max_output_bytes) {
        return Err(VerificationPolicyError(format!(
            "verification check `{}` max_output_bytes must be between 1 and {MAX_OUTPUT_BYTES}",
            config.id
        )));
    }

    let mut path_patterns = config
        .paths
        .iter()
        .map(|path| path.replace('\\', "/"))
        .collect::<Vec<_>>();
    path_patterns.sort();
    path_patterns.dedup();

    let working_directory = confined_directory(root, &config.id, &config.working_directory)?;
    let program = validate_program(root, &config.id, &config.program)?;
    let resolved_program = match &program {
        ValidatedProgram::Bare(program) => check::resolve_bare_program(program, &working_directory),
        ValidatedProgram::RepositoryRelative(program) => Some(program.clone()),
    };
    let executable_sha256 = resolved_program.as_deref().and_then(sha256_file_hex);

    Ok(ValidatedCheck {
        id: config.id.clone(),
        role: config.role,
        program,
        resolved_program,
        executable_sha256,
        args: config.args.clone(),
        working_directory,
        working_directory_label: normalized_label(&config.working_directory),
        timeout_seconds: config.timeout_seconds,
        max_output_bytes: config.max_output_bytes,
        paths: compile_paths(&config.id, &config.paths)?,
        path_patterns,
        cache_enabled: config.cache.enabled,
    })
}

fn validate_id(id: &str) -> Result<(), VerificationPolicyError> {
    let mut chars = id.chars();
    let valid = id.len() <= 64
        && chars
            .next()
            .is_some_and(|ch| ch.is_ascii_lowercase() || ch.is_ascii_digit())
        && chars.all(|ch| ch.is_ascii_lowercase() || ch.is_ascii_digit() || "._-".contains(ch));
    if valid {
        Ok(())
    } else {
        Err(VerificationPolicyError(format!(
            "invalid verification check id `{id}`"
        )))
    }
}

fn validate_program(
    root: &Path,
    id: &str,
    program: &Path,
) -> Result<ValidatedProgram, VerificationPolicyError> {
    if program.is_absolute() || has_parent_component(program) {
        return Err(VerificationPolicyError(format!(
            "verification check `{id}` program must be a bare or repository-relative executable"
        )));
    }
    let components = program.components().collect::<Vec<_>>();
    if components.len() == 1 && matches!(components[0], Component::Normal(_)) {
        return Ok(ValidatedProgram::Bare(
            program.to_string_lossy().into_owned(),
        ));
    }
    let resolved = root.join(program).canonicalize().map_err(|error| {
        VerificationPolicyError(format!(
            "verification check `{id}` program {} cannot be resolved: {error}",
            program.display()
        ))
    })?;
    if !resolved.starts_with(root) || !resolved.is_file() {
        return Err(VerificationPolicyError(format!(
            "verification check `{id}` program must resolve to a file inside the repository"
        )));
    }
    Ok(ValidatedProgram::RepositoryRelative(resolved))
}

fn confined_directory(
    root: &Path,
    id: &str,
    relative: &Path,
) -> Result<PathBuf, VerificationPolicyError> {
    if relative.is_absolute() || has_parent_component(relative) {
        return Err(VerificationPolicyError(format!(
            "verification check `{id}` working_directory must be repository-relative"
        )));
    }
    let resolved = root.join(relative).canonicalize().map_err(|error| {
        VerificationPolicyError(format!(
            "verification check `{id}` working_directory {} cannot be resolved: {error}",
            relative.display()
        ))
    })?;
    if !resolved.starts_with(root) || !resolved.is_dir() {
        return Err(VerificationPolicyError(format!(
            "verification check `{id}` working_directory must resolve to a directory inside the repository"
        )));
    }
    Ok(resolved)
}

fn compile_paths(id: &str, paths: &[String]) -> Result<Option<GlobSet>, VerificationPolicyError> {
    if paths.is_empty() {
        return Ok(None);
    }
    let mut builder = GlobSetBuilder::new();
    for pattern in paths {
        builder.add(Glob::new(pattern).map_err(|error| {
            VerificationPolicyError(format!(
                "verification check `{id}` has invalid path glob `{pattern}`: {error}"
            ))
        })?);
    }
    builder.build().map(Some).map_err(|error| {
        VerificationPolicyError(format!(
            "verification check `{id}` path globs cannot be compiled: {error}"
        ))
    })
}

fn has_parent_component(path: &Path) -> bool {
    path.components()
        .any(|component| matches!(component, Component::ParentDir))
}

fn normalized_label(path: &Path) -> String {
    let label = path.to_string_lossy().replace('\\', "/");
    if label.is_empty() {
        ".".to_string()
    } else {
        label
    }
}
