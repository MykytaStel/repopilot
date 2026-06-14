use crate::audits::code_quality::sanitize::sanitize_c_style;
use crate::audits::traits::FileAudit;
use crate::findings::types::{Confidence, Evidence, Finding, FindingCategory, Severity};
use crate::knowledge::decision::apply_file_decision;
use crate::scan::config::ScanConfig;
use crate::scan::facts::FileFacts;
use std::collections::HashSet;

const SECRET_KEYS: &[&str] = &[
    "api_key",
    "apikey",
    "api_secret",
    "secret_key",
    "secret",
    "password",
    "passwd",
    "private_key",
    "access_key_id",
    "secret_access_key",
    "access_key",
    "auth_token",
    "access_token",
    "refresh_token",
    "token",
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

        // Secret candidates inside an inline `#[cfg(test)]` module are fixtures,
        // not shipped credentials, so skip those lines (the file-path test skip
        // above only catches whole test files).
        let gated_test_lines = rust_cfg_test_lines(file);

        file.content
            .as_deref()
            .unwrap_or("")
            .lines()
            .enumerate()
            .filter_map(|(index, line)| {
                let line_number = index + 1;
                if gated_test_lines.contains(&line_number) {
                    return None;
                }
                detect_secret_line(line, line_number, &file.path).and_then(|finding| {
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

    if let Some(jwt) = find_jwt_like_token(line) {
        return Some(build_finding(
            line_number,
            path,
            "jwt-like token",
            mask_token_in_line(line.trim(), jwt),
        ));
    }

    let lower = line.to_ascii_lowercase();

    if let Some(matched_key) = SECRET_KEYS
        .iter()
        .find(|&&key| assigned_secret_value_for_key(line, &lower, key).is_some())
    {
        return Some(build_finding(
            line_number,
            path,
            matched_key,
            mask_secret_value(line.trim()),
        ));
    }

    None
}

include!("secret_candidate/parsing.rs");
include!("secret_candidate/helpers.rs");
include!("secret_candidate/finding.rs");
