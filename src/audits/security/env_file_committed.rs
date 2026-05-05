use crate::audits::traits::ProjectAudit;
use crate::findings::types::{Evidence, Finding, FindingCategory, Severity};
use crate::scan::config::ScanConfig;
use crate::scan::facts::ScanFacts;

const ENV_FILE_NAMES: &[&str] = &[
    ".env",
    ".env.local",
    ".env.production",
    ".env.staging",
    ".env.development",
];

pub struct EnvFileCommittedAudit;

impl ProjectAudit for EnvFileCommittedAudit {
    fn audit(&self, facts: &ScanFacts, _config: &ScanConfig) -> Vec<Finding> {
        facts
            .files
            .iter()
            .filter(|file| {
                file.path
                    .file_name()
                    .and_then(|n| n.to_str())
                    .map(|name| ENV_FILE_NAMES.contains(&name))
                    .unwrap_or(false)
            })
            .map(|file| build_finding(&file.path))
            .collect()
    }
}

fn build_finding(path: &std::path::Path) -> Finding {
    let name = path
        .file_name()
        .and_then(|n| n.to_str())
        .unwrap_or(".env");

    Finding {
        id: format!("security.env-file-committed.{}", path.display()),
        rule_id: "security.env-file-committed".to_string(),
        title: "Environment file tracked in repository".to_string(),
        description: format!(
            "`{name}` is present in the scanned tree. Environment files often contain secrets and should be listed in `.gitignore`."
        ),
        category: FindingCategory::Security,
        severity: Severity::High,
        evidence: vec![Evidence {
            path: path.to_path_buf(),
            line_start: 1,
            line_end: None,
            snippet: format!("`{name}` should not be committed; add it to .gitignore."),
        }],
    }
}
