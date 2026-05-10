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
    let lower = unquoted.to_lowercase();

    let is_placeholder = unquoted.is_empty()
        || unquoted.starts_with("${")
        || unquoted.starts_with("{{")
        || unquoted.starts_with('<')
        || unquoted.starts_with('[')
        || unquoted.ends_with('…')
        || unquoted.ends_with("...")
        || matches!(
            lower.as_str(),
            "null"
                | "nil"
                | "none"
                | "your_key_here"
                | "changeme"
                | "replace_me"
                | "placeholder"
                | "todo"
                | "xxx"
                | "your_secret"
                | "your_token"
                | "your_api_key"
                | "insert_here"
                | "set_me"
                | "fixme"
        );

    // Known non-secret values: algorithm names, auth scheme keywords, etc.
    const NON_SECRET_VALUES: &[&str] = &[
        "bearer", "rs256", "hs256", "es256", "rs512", "hs512", "es512", "sha256", "sha512", "md5",
        "aes", "aes256", "rsa", "hmac", "basic", "digest", "oauth", "oauth2", "true", "false",
    ];

    if is_placeholder || NON_SECRET_VALUES.contains(&lower.as_str()) || unquoted.len() <= 6 {
        return false;
    }

    // Low-entropy strings (repetitive English words) are not real secrets.
    // Real API keys/tokens have high character diversity (entropy > 3.0 bits/char).
    if unquoted.len() >= 8 && shannon_entropy(unquoted) < 3.0 {
        return false;
    }

    value.starts_with('"')
        || value.starts_with('\'')
        || !unquoted
            .chars()
            .any(|c| c.is_whitespace() || matches!(c, '(' | ')' | '[' | ']' | '{' | '}' | '?'))
}

/// Computes Shannon entropy in bits per character.
fn shannon_entropy(s: &str) -> f64 {
    if s.is_empty() {
        return 0.0;
    }
    let len = s.len() as f64;
    let mut freq = [0u32; 256];
    for &b in s.as_bytes() {
        freq[b as usize] += 1;
    }
    freq.iter()
        .filter(|&&c| c > 0)
        .map(|&c| {
            let p = c as f64 / len;
            -p * p.log2()
        })
        .sum()
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
