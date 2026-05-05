use crate::audits::traits::FileAudit;
use crate::findings::types::{Evidence, Finding, FindingCategory, Severity};
use crate::scan::config::ScanConfig;
use crate::scan::facts::FileFacts;

const PRIVATE_KEY_HEADERS: &[&str] = &[
    "-----BEGIN RSA PRIVATE KEY-----",
    "-----BEGIN EC PRIVATE KEY-----",
    "-----BEGIN PRIVATE KEY-----",
    "-----BEGIN OPENSSH PRIVATE KEY-----",
    "-----BEGIN PGP PRIVATE KEY BLOCK-----",
];

pub struct PrivateKeyCandidateAudit;

impl FileAudit for PrivateKeyCandidateAudit {
    fn audit(&self, file: &FileFacts, _config: &ScanConfig) -> Vec<Finding> {
        if should_skip_private_key_audit(&file.path) {
            return vec![];
        }

        file.content
            .lines()
            .enumerate()
            .filter_map(|(index, line)| {
                let trimmed = line.trim();
                PRIVATE_KEY_HEADERS
                    .iter()
                    .find(|&&header| trimmed.starts_with(header))
                    .map(|header| build_finding(&file.path, index + 1, header))
            })
            .collect()
    }
}

fn should_skip_private_key_audit(path: &std::path::Path) -> bool {
    let lower_path = path.to_string_lossy().to_lowercase();
    if lower_path.contains("test")
        || lower_path.contains("fixture")
        || lower_path.contains("example")
        || lower_path.contains("mock")
    {
        return true;
    }

    let is_markdown = path
        .extension()
        .and_then(|ext| ext.to_str())
        .is_some_and(|ext| ext.eq_ignore_ascii_case("md"));

    is_markdown
        && path
            .components()
            .any(|c| c.as_os_str().to_string_lossy() == "docs")
}

fn build_finding(path: &std::path::Path, line_number: usize, header: &str) -> Finding {
    Finding {
        id: format!(
            "security.private-key-candidate.{}:{}",
            path.display(),
            line_number
        ),
        rule_id: "security.private-key-candidate".to_string(),
        title: "Private key detected in source file".to_string(),
        description: format!(
            "`{}` appears to contain a private key. Private keys must never be committed to version control.",
            path.display()
        ),
        category: FindingCategory::Security,
        severity: Severity::Critical,
        evidence: vec![Evidence {
            path: path.to_path_buf(),
            line_start: line_number,
            line_end: None,
            // Show only the header line — never the actual key bytes
            snippet: header.to_string(),
        }],
    }
}
