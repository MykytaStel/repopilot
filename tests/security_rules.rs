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
fn secret_candidate_skips_docs_and_lock_files() {
    for path in [
        "README.md",
        "docs/security.md",
        "Cargo.lock",
        "pnpm-lock.yaml",
    ] {
        let f = file(path, "API_KEY = \"xK9mQ2pL8rT5vN3wY7\"\n");
        let findings = SecretCandidateAudit.audit(&f, &ScanConfig::default());
        assert!(
            findings.is_empty(),
            "docs and lock files must not be scanned for secrets: {path}"
        );
    }
}

#[test]
fn secret_candidate_detects_and_masks_jwt_like_tokens() {
    let token = "eyJhbGciOiJIUzI1NiJ9.eyJzdWIiOiIxMjM0NTY3ODkwIn0.SflKxwRJSMeKKF2QT4fwpMeJf36POk6yJV_adQssw5c";
    let file = file(
        "src/config.rs",
        &format!("const TOKEN: &str = \"{token}\";\n"),
    );

    let findings = SecretCandidateAudit.audit(&file, &ScanConfig::default());

    assert_eq!(findings.len(), 1);
    assert_eq!(findings[0].rule_id, "security.secret-candidate");
    assert!(findings[0].evidence[0].snippet.contains("eyJhbGci...***"));
    assert!(!findings[0].evidence[0].snippet.contains(token));
}

#[test]
fn secret_candidate_does_not_flag_secret_scanner_variable_names() {
    let file = file(
        "src/audits/security/secret_candidate.rs",
        "let jwt = find_jwt_like_token(line)?;\n",
    );

    let findings = SecretCandidateAudit.audit(&file, &ScanConfig::default());

    assert!(findings.is_empty());
}

#[test]
fn secret_candidate_does_not_flag_variable_references() {
    for line in [
        "api_key=api_key,\n",
        "api_key=settings.openai_api_key,\n",
        "api_key=process.env.OPENAI_API_KEY,\n",
        "password=password,\n",
        "access_token=self.access_token,\n",
    ] {
        let f = file("src/provider.py", line);
        let findings = SecretCandidateAudit.audit(&f, &ScanConfig::default());
        assert!(
            findings.is_empty(),
            "variable reference must not be flagged as a secret: `{line}` -> {findings:?}"
        );
    }
}

#[test]
fn secret_candidate_does_not_flag_shell_expansions() {
    // A captured command output or variable expansion is not a literal secret,
    // even when a closing quote and line continuation follow it.
    for line in [
        "token=\"$(printf '%s' \"$auth_response\" | json_field token)\"\n",
        "  -e POSTGRES_PASSWORD=\"$PGPASSWORD\" \\\n",
        "API_TOKEN=`vault read secret/token`\n",
    ] {
        let f = file("scripts/deploy.sh", line);
        let findings = SecretCandidateAudit.audit(&f, &ScanConfig::default());
        assert!(
            findings.is_empty(),
            "shell expansion must not be flagged as a secret: `{line}` -> {findings:?}"
        );
    }
}

#[test]
fn secret_candidate_skips_inline_cfg_test_module_but_keeps_production() {
    // A fake credential inside an inline `#[cfg(test)]` module is a fixture;
    // a real assignment in the production body of the same file still counts.
    let file = file(
        "src/api/dto/auth.rs",
        "pub const API_KEY: &str = \"abc123xyz987def\";\n\
         #[cfg(test)]\n\
         mod tests {\n\
             fn case() {\n\
                 let password = \"sup3rSekret_inTest_99\";\n\
             }\n\
         }\n",
    );

    let findings = SecretCandidateAudit.audit(&file, &ScanConfig::default());

    assert_eq!(findings.len(), 1, "{findings:?}");
    assert_eq!(findings[0].evidence[0].line_start, 1);
}

#[test]
fn secret_candidate_detects_secret_named_literals() {
    let file = file("src/config.rs", "let jwt = \"abc123xyz987\";\n");

    let findings = SecretCandidateAudit.audit(&file, &ScanConfig::default());

    assert_eq!(findings.len(), 1);
    assert_eq!(findings[0].rule_id, "security.secret-candidate");
    assert!(findings[0].evidence[0].snippet.contains("abc...***"));
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
        files_analyzed: 1,
        ..ScanFacts::default()
    };

    let findings = EnvFileCommittedAudit.audit(&facts, &ScanConfig::default());

    assert_eq!(findings.len(), 1);
    assert_eq!(findings[0].rule_id, "security.env-file-committed");
}

#[test]
fn secret_candidate_skips_ellipsis_truncated_values() {
    // Common in documentation and README examples — clearly not real secrets.
    for line in [
        r#"const API_KEY = "sk_live_…";"#,
        r#"const TOKEN = "eyJhbGci...";"#,
        r#"password = "hunter2...""#,
    ] {
        let f = file("src/config.rs", &format!("{line}\n"));
        let findings = SecretCandidateAudit.audit(&f, &ScanConfig::default());
        assert!(
            findings.is_empty(),
            "ellipsis-truncated value must not be flagged: `{line}` → {findings:?}"
        );
    }
}

#[test]
fn secret_candidate_skips_low_entropy_values() {
    // Low-entropy strings (repetitive English words) look like secrets by keyword alone
    // but their character diversity is too low to be real credentials.
    for line in [
        r#"const SECRET = "testtest";"#,
        r#"password = "aaaaaaaa";"#,
        r#"api_key = "abcabcab";"#,
        r#"password = "password";"#,
        r#"password = "test-password";"#,
        r#"secret = "dummy";"#,
        r#"token = "example";"#,
        r#"api_key = "$PGPASSWORD";"#,
        r#"token = "${TOKEN}";"#,
    ] {
        let f = file("src/config.rs", &format!("{line}\n"));
        let findings = SecretCandidateAudit.audit(&f, &ScanConfig::default());
        assert!(
            findings.is_empty(),
            "low-entropy value must not be flagged: `{line}` → {findings:?}"
        );
    }
}

#[test]
fn secret_candidate_skips_api_key_placeholders_and_shell_examples() {
    for line in [
        r#"echo 'export OPENAI_API_KEY="your-openai-api-key"'"#,
        r#"echo 'export GEMINI_API_KEY="your-gemini-api-key"'"#,
        r#"echo 'export ANTHROPIC_API_KEY="replace-with-anthropic-api-key"'"#,
        r#"const setup = "export OPENAI_API_KEY=<OPENAI_API_KEY>";"#,
        r#"const setup = "export GEMINI_API_KEY=${GEMINI_API_KEY}";"#,
        r#"const setup = "export ANTHROPIC_API_KEY=example-anthropic-api-key";"#,
    ] {
        let f = file(
            "src/features/settings/SettingsTab.tsx",
            &format!("{line}\n"),
        );
        let findings = SecretCandidateAudit.audit(&f, &ScanConfig::default());
        assert!(
            findings.is_empty(),
            "placeholder shell example must not be flagged: `{line}` -> {findings:?}"
        );
    }
}

#[test]
fn secret_candidate_flags_high_entropy_values() {
    // High-entropy strings (mixed case + digits + symbols) are real credentials.
    for line in [
        r#"api_key = "xK9mQ2pL8rT5vN3wY7";"#,
        r#"secret_key = "Zj4Hn8Qw2Kp6Mv9Rs";"#,
        r#"api_key = sk_live_xK9mQ2pL8rT5vN3wY7;"#,
    ] {
        let f = file("src/config.rs", &format!("{line}\n"));
        let findings = SecretCandidateAudit.audit(&f, &ScanConfig::default());
        assert!(
            !findings.is_empty(),
            "high-entropy value must be flagged: `{line}`"
        );
        assert_eq!(
            findings[0].confidence,
            repopilot::findings::types::Confidence::High
        );
    }
}

#[test]
fn secret_candidate_flags_provider_looking_values_after_placeholder_filtering() {
    for line in [
        r#"OPENAI_API_KEY = "sk-proj-xK9mQ2pL8rT5vN3wY7";"#,
        r#"GEMINI_API_KEY = "AIzaSyD4mmyKeyQ7m2vK9pL4xR8tN6zB3w";"#,
        r#"GITHUB_TOKEN = "github_pat_11ABCDEFG0xK9mQ2pL8rT5vN3wY7";"#,
        r#"AWS_ACCESS_KEY_ID = "AKIAIOSFODNN7EXAMPLE";"#,
    ] {
        let f = file("src/config.ts", &format!("{line}\n"));
        let findings = SecretCandidateAudit.audit(&f, &ScanConfig::default());
        assert!(
            !findings.is_empty(),
            "provider-looking value must still be flagged: `{line}`"
        );
        assert_eq!(
            findings[0].confidence,
            repopilot::findings::types::Confidence::High
        );
        assert!(
            !findings[0].evidence[0]
                .snippet
                .contains("xK9mQ2pL8rT5vN3wY7")
        );
    }
}

fn file(path: &str, content: &str) -> FileFacts {
    FileFacts {
        path: PathBuf::from(path),
        language: None,
        non_empty_lines: content.lines().count(),
        branch_count: 0,
        imports: Vec::new(),
        content: Some(content.to_string()),
        has_inline_tests: false,
        in_executable_package: false,
    }
}
