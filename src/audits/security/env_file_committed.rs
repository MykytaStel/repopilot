use crate::audits::security::secret_candidate::line_has_hardcoded_secret;
use crate::audits::traits::ProjectAudit;
use crate::findings::types::{Evidence, Finding, FindingCategory, Severity};
use crate::scan::config::ScanConfig;
use crate::scan::facts::{FileContentProvider, ScanFacts};

// `.env` and `.env.local` are the developer-local secret files that every
// framework's default `.gitignore` excludes — committing one at all is the
// anti-pattern, regardless of its current contents.
const ALWAYS_FLAG_NAMES: &[&str] = &[".env", ".env.local"];

// `.env.development` / `.env.production` / `.env.staging` are routinely committed
// on purpose to carry *public* build configuration (client URLs, feature flags,
// framework-exposed keys). They are only a leak when they actually contain a
// secret-shaped value, so they are content-gated rather than flagged by name.
const CONTENT_GATED_NAMES: &[&str] = &[".env.development", ".env.production", ".env.staging"];

// Env-var prefixes that frameworks inline into client/browser bundles. A value
// under one of these is public by construction (it ships to every visitor), so
// it is not a committed *secret* even when it is high-entropy — e.g. a Firebase
// web API key or a published RSA public key.
const PUBLIC_ENV_PREFIXES: &[&str] = &[
    "VITE_",
    "REACT_APP_",
    "NEXT_PUBLIC_",
    "NUXT_PUBLIC_",
    "PUBLIC_",
    "EXPO_PUBLIC_",
    "GATSBY_",
    "VUE_APP_",
    "STORYBOOK_",
];

#[derive(Clone, Copy)]
enum EnvTier {
    AlwaysFlag,
    ContentGated,
}

pub struct EnvFileCommittedAudit;

impl ProjectAudit for EnvFileCommittedAudit {
    fn audit(&self, facts: &ScanFacts, _config: &ScanConfig) -> Vec<Finding> {
        facts
            .files
            .iter()
            .filter_map(|file| {
                let name = file.path.file_name().and_then(|n| n.to_str())?;
                match env_file_tier(name)? {
                    EnvTier::AlwaysFlag => Some(build_finding(&file.path, EnvTier::AlwaysFlag)),
                    EnvTier::ContentGated => {
                        // Project audits run after file content is dropped, so
                        // re-read from disk. Without readable contents we cannot
                        // prove the file is safe, so fall back to flagging rather
                        // than hide a possible leak.
                        let has_secret = FileContentProvider
                            .content(file)
                            .map(|content| content_has_committed_secret(&content))
                            .unwrap_or(true);
                        has_secret.then(|| build_finding(&file.path, EnvTier::ContentGated))
                    }
                }
            })
            .collect()
    }
}

fn env_file_tier(name: &str) -> Option<EnvTier> {
    if ALWAYS_FLAG_NAMES.contains(&name) {
        Some(EnvTier::AlwaysFlag)
    } else if CONTENT_GATED_NAMES.contains(&name) {
        Some(EnvTier::ContentGated)
    } else {
        None
    }
}

// True when any non-comment, non-public-prefixed line carries a secret-shaped
// assignment (using the same value classifier as `security.secret-candidate`).
fn content_has_committed_secret(content: &str) -> bool {
    content.lines().any(|line| {
        let trimmed = line.trim_start();
        if trimmed.is_empty() || trimmed.starts_with('#') {
            return false;
        }
        if env_key(trimmed).is_some_and(is_public_env_key) {
            return false;
        }
        line_has_hardcoded_secret(line)
    })
}

fn env_key(line: &str) -> Option<&str> {
    let (key, _) = line.split_once('=')?;
    Some(key.trim().trim_start_matches("export ").trim())
}

fn is_public_env_key(key: &str) -> bool {
    let upper = key.to_ascii_uppercase();
    PUBLIC_ENV_PREFIXES
        .iter()
        .any(|prefix| upper.starts_with(prefix))
}

fn build_finding(path: &std::path::Path, tier: EnvTier) -> Finding {
    let name = path.file_name().and_then(|n| n.to_str()).unwrap_or(".env");

    let description = match tier {
        EnvTier::AlwaysFlag => format!(
            "`{name}` is present in the scanned tree. Local environment files commonly hold secrets and should be listed in `.gitignore`."
        ),
        EnvTier::ContentGated => format!(
            "`{name}` is committed and contains a secret-shaped value. Move the secret to an untracked `.env.local` and keep only public build configuration here."
        ),
    };

    Finding {
        id: String::new(),
        rule_id: "security.env-file-committed".to_string(),
        recommendation: Finding::recommendation_for_rule_id("security.env-file-committed"),
        title: "Environment file tracked in repository".to_string(),
        description,
        category: FindingCategory::Security,
        severity: Severity::Critical,
        confidence: Default::default(),
        evidence: vec![Evidence {
            path: path.to_path_buf(),
            line_start: 1,
            line_end: None,
            snippet: format!("`{name}` should not be committed; add it to .gitignore."),
        }],
        workspace_package: None,
        docs_url: None,
        provenance: Default::default(),
        risk: Default::default(),
    }
}
