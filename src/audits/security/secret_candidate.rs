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
    "client_secret",
    "signing_key",
    "encryption_key",
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

fn detect_secret_line(
    line: &str,
    line_number: usize,
    path: &std::path::Path,
) -> Option<Finding> {
    let lower = line.to_lowercase();

    let matched_key = SECRET_KEYS.iter().find(|&&key| {
        if !lower.contains(key) {
            return false;
        }
        // Must be followed by = or : and a non-empty quoted or unquoted value
        let after_key = lower.split(key).nth(1).unwrap_or_default().trim_start();
        let starts_assignment = after_key.starts_with('=') || after_key.starts_with(':');
        if !starts_assignment {
            return false;
        }
        let value = after_key[1..].trim();
        // Skip empty values, placeholders, and template vars
        let is_placeholder = value.is_empty()
            || value.starts_with("${")
            || value.starts_with("{{")
            || value == "\"\"" || value == "''"
            || value == "null" || value == "nil"
            || value == "none" || value == "your_key_here"
            || value == "changeme";
        !is_placeholder && value.len() > 4
    })?;

    Some(Finding {
        id: format!("security.secret-candidate.{}:{}", path.display(), line_number),
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
            snippet: mask_secret_value(line.trim()),
        }],
    })
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
