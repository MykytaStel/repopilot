use crate::findings::types::{Confidence, FindingCategory, Severity};
use crate::rules::metadata::RuleMetadata;
use crate::rules::{RuleLifecycle, SignalSource};

pub(super) static RULES: &[RuleMetadata] = &[
    RuleMetadata {
        rule_id: "security.env-file-committed",
        title: "Environment file committed to version control",
        category: FindingCategory::Security,
        default_severity: Severity::Critical,
        default_confidence: Confidence::High,
        lifecycle: RuleLifecycle::Stable,
        signal_source: SignalSource::ConfigFile,
        docs_url: Some("https://12factor.net/config"),
        description: "A .env file containing environment variables has been committed. These files frequently contain secrets, API keys, or credentials that should never enter version control.",
        recommendation: Some(
            "Add .env (and .env.*) to .gitignore, rotate any exposed credentials immediately, and use a secrets manager or CI environment variables instead.",
        ),
        ..RuleMetadata::DEFAULT
    },
    RuleMetadata {
        rule_id: "security.private-key-candidate",
        title: "Possible private key in source file",
        category: FindingCategory::Security,
        default_severity: Severity::Critical,
        default_confidence: Confidence::High,
        lifecycle: RuleLifecycle::Stable,
        signal_source: SignalSource::TextHeuristic,
        docs_url: Some(
            "https://github.com/MykytaStel/repopilot/blob/main/docs/security.md#reporting-a-security-issue",
        ),
        description: "A PEM-encoded private key block was found in a source file. Committed private keys can be extracted from git history even after deletion.",
        recommendation: Some(
            "Remove the key from the file immediately, rotate the key pair, and purge the git history using git-filter-repo or BFG Repo Cleaner.",
        ),
        ..RuleMetadata::DEFAULT
    },
    RuleMetadata {
        rule_id: "security.secret-candidate",
        title: "Possible hardcoded secret or API key",
        category: FindingCategory::Security,
        default_severity: Severity::High,
        default_confidence: Confidence::Low,
        lifecycle: RuleLifecycle::Preview,
        signal_source: SignalSource::TextHeuristic,
        docs_url: Some(
            "https://github.com/MykytaStel/repopilot/blob/main/docs/security.md#secret-handling",
        ),
        description: "A high-entropy string or a pattern matching a known secret format was found in source code. Hardcoded secrets are exposed to everyone with repository access.",
        recommendation: Some(
            "Move the value to an environment variable or secrets manager. If already committed, rotate the credential and consider the old value compromised.",
        ),
        ..RuleMetadata::DEFAULT
    },
];
