use repopilot::audits::security::env_file_committed::EnvFileCommittedAudit;
use repopilot::audits::security::private_key_candidate::PrivateKeyCandidateAudit;
use repopilot::audits::security::secret_candidate::SecretCandidateAudit;
use repopilot::audits::traits::{FileAudit, ProjectAudit};
use repopilot::scan::config::ScanConfig;
use repopilot::scan::facts::{FileFacts, ScanFacts};
use std::path::PathBuf;

#[test]
fn secret_candidate_masks_secret_values_and_skips_placeholders() {
    let file = file(
        "src/config.rs",
        "API_KEY = \"abc123xyz987\"\npassword = \"\"\ntoken = \"${TOKEN}\"\n",
    );

    let findings = SecretCandidateAudit.audit(&file, &ScanConfig::default());

    assert_eq!(findings.len(), 1);
    assert_eq!(findings[0].rule_id, "security.secret-candidate");
    assert!(findings[0].evidence[0].snippet.contains("abc...***"));
    assert!(!findings[0].evidence[0].snippet.contains("abc123xyz987"));
}

#[test]
fn secret_candidate_skips_test_and_fixture_paths() {
    let file = file("tests/fixture.rs", "API_KEY = \"abc123xyz987\"\n");

    let findings = SecretCandidateAudit.audit(&file, &ScanConfig::default());

    assert!(findings.is_empty());
}

#[test]
fn private_key_candidate_reports_header_without_key_body() {
    let file = file(
        "src/key.pem",
        "-----BEGIN RSA PRIVATE KEY-----\nvery-secret-key-bytes\n-----END RSA PRIVATE KEY-----\n",
    );

    let findings = PrivateKeyCandidateAudit.audit(&file, &ScanConfig::default());

    assert_eq!(findings.len(), 1);
    assert_eq!(findings[0].rule_id, "security.private-key-candidate");
    assert_eq!(
        findings[0].evidence[0].snippet,
        "-----BEGIN RSA PRIVATE KEY-----"
    );
    assert!(
        !findings[0].evidence[0]
            .snippet
            .contains("very-secret-key-bytes")
    );
}

#[test]
fn private_key_candidate_ignores_documented_examples() {
    let file = file(
        "docs/rulesets.md",
        "Example: -----BEGIN RSA PRIVATE KEY-----\n",
    );

    let findings = PrivateKeyCandidateAudit.audit(&file, &ScanConfig::default());

    assert!(findings.is_empty());
}

#[test]
fn env_file_committed_reports_env_files() {
    let facts = ScanFacts {
        root_path: PathBuf::from("demo"),
        files: vec![file(".env.production", "TOKEN=value\n")],
        files_count: 1,
        ..ScanFacts::default()
    };

    let findings = EnvFileCommittedAudit.audit(&facts, &ScanConfig::default());

    assert_eq!(findings.len(), 1);
    assert_eq!(findings[0].rule_id, "security.env-file-committed");
}

fn file(path: &str, content: &str) -> FileFacts {
    FileFacts {
        path: PathBuf::from(path),
        language: None,
        lines_of_code: content.lines().count(),
        content: content.to_string(),
    }
}
