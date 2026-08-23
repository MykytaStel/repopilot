use crate::scan::session::WorkspaceRevision;
use crate::verification::VerificationRole;
use crate::verification::executor::INHERITED_ENV_KEYS;
use crate::verification::policy::{ValidatedCheck, ValidatedProgram};
use serde::Serialize;
use sha2::{Digest, Sha256};
use std::collections::BTreeMap;
use std::ffi::OsStr;
use std::fs::File;
use std::io::Read;
use std::path::{Path, PathBuf};

pub(super) const CACHE_SCHEMA_VERSION: u32 = 1;

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct VerificationCacheKey(pub(super) String);

impl VerificationCacheKey {
    pub(crate) fn build(check: &ValidatedCheck, revision: &WorkspaceRevision) -> Option<Self> {
        let context = production_context();
        build_with_context(check, revision, &context)
    }

    pub(crate) fn as_str(&self) -> &str {
        &self.0
    }
}

struct KeyContext<'a> {
    schema_version: u32,
    repopilot_version: &'a str,
    os: &'a str,
    arch: &'a str,
    inherited_environment: BTreeMap<String, Option<Vec<u8>>>,
}

#[derive(Serialize)]
struct KeyInput<'a> {
    schema_version: u32,
    repopilot_version: &'a str,
    os: &'a str,
    arch: &'a str,
    workspace_revision: &'a str,
    check: CheckInput<'a>,
    executable_sha256: String,
    inherited_environment_sha256: String,
}

#[derive(Serialize)]
struct CheckInput<'a> {
    id: &'a str,
    role: VerificationRole,
    program: String,
    args: &'a [String],
    working_directory: &'a str,
    timeout_seconds: u64,
    max_output_bytes: usize,
    path_patterns: &'a [String],
}

fn build_with_context(
    check: &ValidatedCheck,
    revision: &WorkspaceRevision,
    context: &KeyContext<'_>,
) -> Option<VerificationCacheKey> {
    let executable = resolved_executable(&check.program)?;
    let executable_sha256 = hash_file(&executable)?;
    let input = KeyInput {
        schema_version: context.schema_version,
        repopilot_version: context.repopilot_version,
        os: context.os,
        arch: context.arch,
        workspace_revision: revision.id(),
        check: CheckInput {
            id: check.id(),
            role: check.role,
            program: program_label(&check.program),
            args: &check.args,
            working_directory: &check.working_directory_label,
            timeout_seconds: check.timeout_seconds,
            max_output_bytes: check.max_output_bytes,
            path_patterns: &check.path_patterns,
        },
        executable_sha256,
        inherited_environment_sha256: hash_environment(&context.inherited_environment),
    };
    let bytes = serde_json::to_vec(&input).ok()?;
    Some(VerificationCacheKey(hex(&Sha256::digest(bytes))))
}

fn production_context() -> KeyContext<'static> {
    let inherited_environment = INHERITED_ENV_KEYS
        .iter()
        .map(|key| {
            (
                (*key).to_string(),
                std::env::var_os(key).map(|value| os_value_bytes(&value)),
            )
        })
        .collect();
    KeyContext {
        schema_version: CACHE_SCHEMA_VERSION,
        repopilot_version: env!("CARGO_PKG_VERSION"),
        os: std::env::consts::OS,
        arch: std::env::consts::ARCH,
        inherited_environment,
    }
}

fn program_label(program: &ValidatedProgram) -> String {
    match program {
        ValidatedProgram::Bare(program) => program.clone(),
        ValidatedProgram::RepositoryRelative(program) => {
            program.to_string_lossy().replace('\\', "/")
        }
    }
}

fn resolved_executable(program: &ValidatedProgram) -> Option<PathBuf> {
    match program {
        ValidatedProgram::RepositoryRelative(path) => Some(path.clone()),
        ValidatedProgram::Bare(program) => resolve_bare_program(program),
    }
}

fn resolve_bare_program(program: &str) -> Option<PathBuf> {
    let path = std::env::var_os("PATH")?;
    let extensions = std::env::var("PATHEXT").ok();
    let candidates = program_candidates(program, extensions.as_deref(), cfg!(windows));
    std::env::split_paths(&path)
        .flat_map(|directory| candidates.iter().map(move |name| directory.join(name)))
        .find(|candidate| candidate.is_file())
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

fn hash_file(path: &Path) -> Option<String> {
    let mut file = File::open(path).ok()?;
    let mut hasher = Sha256::new();
    let mut buffer = [0_u8; 16_384];
    loop {
        let read = file.read(&mut buffer).ok()?;
        if read == 0 {
            break;
        }
        hasher.update(&buffer[..read]);
    }
    Some(hex(&hasher.finalize()))
}

fn hash_environment(environment: &BTreeMap<String, Option<Vec<u8>>>) -> String {
    let mut hasher = Sha256::new();
    for (key, value) in environment {
        hasher.update((key.len() as u64).to_le_bytes());
        hasher.update(key.as_bytes());
        match value {
            Some(value) => {
                hasher.update([1]);
                hasher.update((value.len() as u64).to_le_bytes());
                hasher.update(value);
            }
            None => hasher.update([0]),
        }
    }
    hex(&hasher.finalize())
}

#[cfg(unix)]
fn os_value_bytes(value: &OsStr) -> Vec<u8> {
    use std::os::unix::ffi::OsStrExt;
    value.as_bytes().to_vec()
}

#[cfg(windows)]
fn os_value_bytes(value: &OsStr) -> Vec<u8> {
    use std::os::windows::ffi::OsStrExt;
    value.encode_wide().flat_map(u16::to_le_bytes).collect()
}

#[cfg(not(any(unix, windows)))]
fn os_value_bytes(value: &OsStr) -> Vec<u8> {
    value.to_string_lossy().as_bytes().to_vec()
}

fn hex(bytes: &[u8]) -> String {
    const DIGITS: &[u8; 16] = b"0123456789abcdef";
    let mut output = String::with_capacity(bytes.len() * 2);
    for byte in bytes {
        output.push(DIGITS[(byte >> 4) as usize] as char);
        output.push(DIGITS[(byte & 0x0f) as usize] as char);
    }
    output
}

#[cfg(test)]
mod tests;
