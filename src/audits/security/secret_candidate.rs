use crate::audits::traits::FileAudit;
use crate::findings::types::{Confidence, Evidence, Finding, FindingCategory, Severity};
use crate::knowledge::decision::apply_file_decision;
use crate::scan::config::ScanConfig;
use crate::scan::facts::FileFacts;

const SECRET_KEYS: &[&str] = &[
    "api_key",
    "apikey",
    "api_secret",
    "secret_key",
    "secret",
    "password",
    "passwd",
    "private_key",
    "auth_token",
    "access_token",
    "refresh_token",
    "client_secret",
    "signing_key",
    "encryption_key",
    "bearer",
    "jwt",
];

pub struct SecretCandidateAudit;

impl FileAudit for SecretCandidateAudit {
    fn audit(&self, file: &FileFacts, _config: &ScanConfig) -> Vec<Finding> {
        let lower_path = file.path.to_string_lossy().to_lowercase();

        // Skip documentation — markdown may reference secrets in code samples, not real values
        let ext = file
            .path
            .extension()
            .and_then(|e| e.to_str())
            .unwrap_or_default();
        if matches!(ext, "md" | "mdx" | "rst" | "txt") {
            return vec![];
        }

        // Skip lock files — they contain high-entropy integrity hashes (sha512, etc.)
        // that are checksums, not secrets. Covers all major package managers.
        if is_lock_file(&file.path) {
            return vec![];
        }

        // Skip test and example files — likely contain fake credentials intentionally
        if lower_path.contains("test")
            || lower_path.contains("fixture")
            || lower_path.contains("example")
            || lower_path.contains("mock")
        {
            return vec![];
        }

        file.content
            .as_deref()
            .unwrap_or("")
            .lines()
            .enumerate()
            .filter_map(|(index, line)| {
                detect_secret_line(line, index + 1, &file.path).and_then(|finding| {
                    apply_file_decision("security.secret-candidate", file, finding, None)
                })
            })
            .collect()
    }
}

fn is_lock_file(path: &std::path::Path) -> bool {
    let ext = path
        .extension()
        .and_then(|e| e.to_str())
        .unwrap_or_default();
    if matches!(ext, "lock" | "lockb") {
        return true;
    }
    let name = path
        .file_name()
        .and_then(|n| n.to_str())
        .unwrap_or_default();
    matches!(
        name,
        "package-lock.json"
            | "pnpm-lock.yaml"
            | "pnpm-lock.yml"
            | "go.sum"
            | "go.work.sum"
            | "bun.lock"
    )
}

fn detect_secret_line(line: &str, line_number: usize, path: &std::path::Path) -> Option<Finding> {
    // Skip PEM headers — PrivateKeyCandidateAudit handles these (avoids double-reporting)
    if line.trim_start().starts_with("-----BEGIN") {
        return None;
    }

    let lower = line.to_lowercase();

    if let Some(matched_key) = SECRET_KEYS.iter().find(|&&key| {
        if !lower.contains(key) {
            return false;
        }
        let after_key = lower
            .split_once(key)
            .map(|(_, after_key)| after_key)
            .unwrap_or_default()
            .trim_start();
        assigned_value_after_key(after_key).is_some_and(is_secret_literal)
    }) {
        return Some(build_finding(
            line_number,
            path,
            matched_key,
            mask_secret_value(line.trim()),
        ));
    }

    let jwt = find_jwt_like_token(line)?;
    Some(build_finding(
        line_number,
        path,
        "jwt-like token",
        mask_token_in_line(line.trim(), jwt),
    ))
}

include!("secret_candidate/helpers.rs");
