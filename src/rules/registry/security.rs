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
        max_confidence: Confidence::High,
        contextual_confidence: true,
        lifecycle: RuleLifecycle::Stable,
        signal_source: SignalSource::ConfigFile,
        docs_url: Some("https://12factor.net/config"),
        description: "A local `.env`/`.env.local` file or a shared `.env.*` variant with credential-shaped content was committed. Local env files commonly hold secrets; shared build env files may hold public browser configuration but still require content inspection.",
        recommendation: Some(
            "Keep `.env` and `.env.local` untracked, rotate any exposed credentials, and move real secrets to a secrets manager or CI environment variables. Shared `.env.development`/`.env.production` files may contain public build config, but public-prefixed variables must not contain server-side passwords, private keys, auth tokens, or client secrets.",
        ),
        false_positive_notes: Some(
            "`.env.example`-style sample files are allowed. `.env` and `.env.local` are treated as unsafe to commit even if their current contents look harmless. Shared build variants such as `.env.production` are content-aware: public browser config can be low confidence or skipped, but explicit sensitive keys and non-public secret-shaped assignments stay high confidence.",
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
        false_positive_notes: Some(
            "Documentation examples and placeholder PEM blocks should live in docs or fixture paths and should not contain real key material.",
        ),
        ..RuleMetadata::DEFAULT
    },
    RuleMetadata {
        rule_id: "security.secret-candidate",
        title: "Possible hardcoded secret or API key",
        category: FindingCategory::Security,
        default_severity: Severity::High,
        default_confidence: Confidence::Medium,
        max_confidence: Confidence::High,
        contextual_confidence: true,
        lifecycle: RuleLifecycle::Preview,
        signal_source: SignalSource::TextHeuristic,
        docs_url: Some(
            "https://github.com/MykytaStel/repopilot/blob/main/docs/security.md#secret-handling",
        ),
        description: "A high-entropy string or a pattern matching a known secret format was found in source code. Hardcoded secrets are exposed to everyone with repository access.",
        recommendation: Some(
            "Move the value to an environment variable or secrets manager. If this is a real credential that was committed, rotate it and consider the old value compromised. Use explicit placeholders such as `<OPENAI_API_KEY>` or `${OPENAI_API_KEY}` in examples.",
        ),
        false_positive_notes: Some(
            "Public labels, documented variable names, environment-variable references, and placeholders such as `your-openai-api-key`, `replace-with-*`, `example-*`, `<OPENAI_API_KEY>`, and `${OPENAI_API_KEY}` should not trigger; entropy and provider-looking token formats are both considered. High-entropy tokens that appear inside a URL value (e.g. Firebase Storage download URLs with `?token=` query parameters) are also skipped — they are URL parameters, not hardcoded secrets. Files under paths containing `tutorial` are treated as example/docs code and skipped. Environment-variable name strings such as `envvar=\"GITHUB_TOKEN\"` (all-caps identifiers referencing an env var) are not flagged.",
        ),
        ..RuleMetadata::DEFAULT
    },
];
