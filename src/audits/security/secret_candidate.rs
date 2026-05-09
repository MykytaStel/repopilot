use crate::audits::traits::FileAudit;
use crate::findings::types::{Evidence, Finding, FindingCategory, Severity};
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

        // Skip test and example files — likely contain fake credentials intentionally
        if lower_path.contains("test")
            || lower_path.contains("fixture")
            || lower_path.contains("example")
            || lower_path.contains("mock")
        {
            return vec![];
        }

        file.content
            .lines()
            .enumerate()
            .filter_map(|(index, line)| detect_secret_line(line, index + 1, &file.path))
            .collect()
    }
}

fn detect_secret_line(line: &str, line_number: usize, path: &std::path::Path) -> Option<Finding> {
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

fn assigned_value_after_key(after_key: &str) -> Option<&str> {
    let trimmed = after_key.trim_start();

    if let Some(value) = trimmed.strip_prefix('=') {
        return Some(value.trim_start());
    }

    let after_colon = trimmed.strip_prefix(':')?.trim_start();
    if let Some((_, value)) = after_colon.split_once('=') {
        return Some(value.trim_start());
    }

    Some(after_colon)
}

fn is_secret_literal(value: &str) -> bool {
    let value = value.trim().trim_end_matches([',', ';']).trim();
    let unquoted = value.trim_matches('"').trim_matches('\'');

    let is_placeholder = unquoted.is_empty()
        || unquoted.starts_with("${")
        || unquoted.starts_with("{{")
        || unquoted.starts_with('<')
        || matches!(
            unquoted,
            "null" | "nil" | "none" | "your_key_here" | "changeme"
        );

    if is_placeholder || unquoted.len() <= 4 {
        return false;
    }

    value.starts_with('"')
        || value.starts_with('\'')
        || !unquoted
            .chars()
            .any(|c| c.is_whitespace() || matches!(c, '(' | ')' | '[' | ']' | '{' | '}' | '?'))
}

fn build_finding(
    line_number: usize,
    path: &std::path::Path,
    matched_key: &str,
    snippet: String,
) -> Finding {
    Finding {
        id: String::new(),
        rule_id: "security.secret-candidate".to_string(),
        title: "Possible secret detected".to_string(),
        description: format!(
            "Line {line_number} in `{}` looks like it may contain a hardcoded secret (matched key: `{matched_key}`). Review and move to environment variables or a secrets manager.",
            path.display()
        ),
        category: FindingCategory::Security,
        severity: Severity::High,
        evidence: vec![Evidence {
            path: path.to_path_buf(),
            line_start: line_number,
            line_end: None,
            snippet,
        }],
        workspace_package: None,
        docs_url: None,
    }
}

fn find_jwt_like_token(line: &str) -> Option<&str> {
    line.split(|c: char| !c.is_ascii_alphanumeric() && c != '-' && c != '_' && c != '.')
        .find(|candidate| is_jwt_like_token(candidate))
}

fn is_jwt_like_token(candidate: &str) -> bool {
    if candidate.len() < 40 || !candidate.starts_with("eyJ") {
        return false;
    }

    let parts: Vec<_> = candidate.split('.').collect();
    parts.len() == 3
        && parts
            .iter()
            .all(|part| part.len() >= 8 && part.chars().all(is_base64url_char))
}

fn is_base64url_char(c: char) -> bool {
    c.is_ascii_alphanumeric() || c == '-' || c == '_'
}

fn mask_token_in_line(line: &str, token: &str) -> String {
    let visible_prefix = token.chars().take(8).collect::<String>();
    line.replace(token, &format!("{visible_prefix}...***"))
}

fn mask_secret_value(line: &str) -> String {
    // Find the first = or : assignment and mask everything after the first 3 chars of value
    if let Some(pos) = line.find('=').or_else(|| line.find(':')) {
        let (key_part, value_part) = line.split_at(pos + 1);
        let value = value_part.trim().trim_matches('"').trim_matches('\'');
        if value.len() > 3 {
            return format!("{key_part} \"{}...***\"", &value[..3]);
        }
    }
    format!("{line} [value masked]")
}
