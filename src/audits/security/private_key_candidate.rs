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
        file.content
            .lines()
            .enumerate()
            .filter_map(|(index, line)| {
                let trimmed = line.trim();
                PRIVATE_KEY_HEADERS
                    .iter()
                    .find(|&&header| trimmed.contains(header))
                    .map(|header| build_finding(&file.path, index + 1, header))
            })
            .collect()
    }
}

fn build_finding(path: &std::path::Path, line_number: usize, header: &str) -> Finding {
    Finding {
        id: format!("security.private-key-candidate.{}:{}", path.display(), line_number),
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
