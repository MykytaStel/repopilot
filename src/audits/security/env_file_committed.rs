use crate::audits::security::secret_candidate::secret_value_confidence;
use crate::audits::traits::ProjectAudit;
use crate::findings::types::{Confidence, Evidence, Finding, FindingCategory, Severity};
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

// Env-var prefixes that frameworks inline into client/browser bundles. Prefix
// recognition is case-sensitive: `VITE_...` is framework-public, but `vite_...`
// is just an ordinary variable name.
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

const EXPLICIT_SENSITIVE_KEY_PARTS: &[&str] = &[
    "PASSWORD",
    "PASSWD",
    "CLIENT_SECRET",
    "AUTH_TOKEN",
    "ACCESS_TOKEN",
    "REFRESH_TOKEN",
    "PRIVATE_KEY",
    "SECRET_KEY",
    "SECRET_ACCESS_KEY",
    "STRIPE_SECRET",
];

const SECRET_LIKE_KEY_PARTS: &[&str] = &[
    "API_KEY",
    "APIKEY",
    "API_SECRET",
    "SECRET",
    "PASSWORD",
    "PASSWD",
    "PRIVATE_KEY",
    "ACCESS_KEY_ID",
    "SECRET_ACCESS_KEY",
    "ACCESS_KEY",
    "AUTH_TOKEN",
    "ACCESS_TOKEN",
    "REFRESH_TOKEN",
    "TOKEN",
    "CLIENT_SECRET",
    "SIGNING_KEY",
    "ENCRYPTION_KEY",
    "BEARER",
    "JWT",
];

const SENSITIVE_URL_PARAM_KEYS: &[&str] = &[
    "password",
    "passwd",
    "client_secret",
    "auth_token",
    "access_token",
    "refresh_token",
    "private_key",
    "secret_key",
    "api_key",
    "apikey",
    "token",
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
                    EnvTier::AlwaysFlag => Some(build_finding(
                        &file.path,
                        EnvTier::AlwaysFlag,
                        Confidence::High,
                    )),
                    EnvTier::ContentGated => {
                        // Project audits run after file content is dropped, so
                        // re-read from disk. Without readable contents we cannot
                        // prove the file is safe, so fall back to flagging rather
                        // than hide a possible leak.
                        match FileContentProvider.content(file) {
                            Some(content) => {
                                committed_secret_confidence(&content).map(|confidence| {
                                    build_finding(&file.path, EnvTier::ContentGated, confidence)
                                })
                            }
                            None => Some(build_finding(
                                &file.path,
                                EnvTier::ContentGated,
                                Confidence::High,
                            )),
                        }
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

fn committed_secret_confidence(content: &str) -> Option<Confidence> {
    content.lines().filter_map(env_line_secret_confidence).max()
}

fn env_line_secret_confidence(line: &str) -> Option<Confidence> {
    let trimmed = line.trim_start();
    if trimmed.is_empty() || trimmed.starts_with('#') {
        return None;
    }

    let (key, value) = env_assignment(trimmed)?;
    env_assignment_confidence(key, value)
}

fn env_assignment_confidence(key: &str, value: &str) -> Option<Confidence> {
    if is_placeholder_or_reference(value) {
        return None;
    }

    if url_secret_confidence(value).is_some() {
        return Some(Confidence::High);
    }

    if is_safe_public_scalar_value(value) {
        return None;
    }

    if is_known_server_provider_token(value) {
        return Some(Confidence::High);
    }

    let value_confidence = credential_shaped_value_confidence(value);
    let public_key = is_public_env_key(key);

    if public_key && is_explicit_sensitive_key(key) {
        return value_confidence.map(|_| Confidence::High);
    }

    if public_key && is_public_credential_config_key(key) {
        return value_confidence.map(|_| Confidence::Low);
    }

    if public_key {
        return None;
    }

    if is_secret_like_key(key) {
        return value_confidence.map(|_| Confidence::High);
    }

    None
}

fn env_assignment(line: &str) -> Option<(&str, &str)> {
    let (key, value) = line.split_once('=')?;
    Some((
        key.trim().trim_start_matches("export ").trim(),
        value.trim(),
    ))
}

fn credential_shaped_value_confidence(value: &str) -> Option<Confidence> {
    secret_value_confidence(value).or_else(|| {
        quoted_segments(value)
            .filter_map(secret_value_confidence)
            .max()
    })
}

fn is_safe_public_scalar_value(value: &str) -> bool {
    let value = unquote_env_value(value);
    let lower = value.to_ascii_lowercase();
    lower == "true" || lower == "false" || value.parse::<f64>().is_ok() || is_safe_http_url(value)
}

fn is_placeholder_or_reference(value: &str) -> bool {
    let value = unquote_env_value(value);
    value.is_empty()
        || value.starts_with('$')
        || value.starts_with("${")
        || value.starts_with("{{")
        || value.starts_with('<')
        || value.starts_with('[')
}

fn quoted_segments(value: &str) -> impl Iterator<Item = &str> {
    value
        .split(['"', '\''])
        .enumerate()
        .filter_map(|(index, segment)| (index % 2 == 1).then_some(segment))
}

fn unquote_env_value(value: &str) -> &str {
    value.trim().trim_matches('"').trim_matches('\'')
}

fn is_safe_http_url(value: &str) -> bool {
    let Some(url) = http_url(value) else {
        return false;
    };
    url_secret_confidence(url).is_none()
}

fn url_secret_confidence(value: &str) -> Option<Confidence> {
    let url = http_url(value)?;
    let after_scheme = url
        .strip_prefix("https://")
        .or_else(|| url.strip_prefix("http://"))?;
    let authority = after_scheme
        .split(['/', '?', '#'])
        .next()
        .unwrap_or_default();
    if authority.contains('@') {
        return Some(Confidence::High);
    }

    let query = url.split_once('?')?.1.split('#').next().unwrap_or_default();
    query.split('&').find_map(|pair| {
        let (key, value) = pair.split_once('=')?;
        (is_sensitive_url_param_key(key) && is_substantial_literal(value))
            .then_some(Confidence::High)
    })
}

fn http_url(value: &str) -> Option<&str> {
    let value = unquote_env_value(value);
    let lower = value.to_ascii_lowercase();
    (lower.starts_with("https://") || lower.starts_with("http://")).then_some(value)
}

fn is_sensitive_url_param_key(key: &str) -> bool {
    let key = key.to_ascii_lowercase();
    SENSITIVE_URL_PARAM_KEYS
        .iter()
        .any(|part| key.contains(part))
}

fn is_substantial_literal(value: &str) -> bool {
    let value = unquote_env_value(value);
    !is_placeholder_or_reference(value) && value.len() >= 8
}

fn is_known_server_provider_token(value: &str) -> bool {
    let value = unquote_env_value(value);
    value.starts_with("sk-")
        || value.starts_with("sk_live_")
        || value.starts_with("sk_test_")
        || value.starts_with("ghp_")
        || value.starts_with("github_pat_")
}

fn is_public_env_key(key: &str) -> bool {
    PUBLIC_ENV_PREFIXES
        .iter()
        .any(|prefix| key.starts_with(prefix))
}

fn is_public_credential_config_key(key: &str) -> bool {
    key_contains_any(key, &["CONFIG", "API_KEY", "APIKEY", "KEY"])
}

fn is_explicit_sensitive_key(key: &str) -> bool {
    key_contains_any(key, EXPLICIT_SENSITIVE_KEY_PARTS)
}

fn is_secret_like_key(key: &str) -> bool {
    key_contains_any(key, SECRET_LIKE_KEY_PARTS)
}

fn key_contains_any(key: &str, parts: &[&str]) -> bool {
    let normalized = key.to_ascii_uppercase();
    parts.iter().any(|part| {
        normalized == *part
            || normalized
                .strip_suffix(part)
                .is_some_and(|prefix| prefix.ends_with('_'))
    })
}

fn build_finding(path: &std::path::Path, tier: EnvTier, confidence: Confidence) -> Finding {
    let name = path.file_name().and_then(|n| n.to_str()).unwrap_or(".env");

    let description = match tier {
        EnvTier::AlwaysFlag => format!(
            "`{name}` is present in the scanned tree. Local environment files commonly hold secrets and should be listed in `.gitignore`."
        ),
        EnvTier::ContentGated => match confidence {
            Confidence::Low => format!(
                "`{name}` is committed and contains public-prefixed browser configuration with a credential-shaped value. Confirm it is intentionally public and not a server-side secret."
            ),
            Confidence::Medium | Confidence::High => format!(
                "`{name}` is committed and contains a secret-shaped value. Move the secret to an untracked `.env.local` and keep only public build configuration here."
            ),
        },
    };

    Finding {
        id: String::new(),
        rule_id: "security.env-file-committed".to_string(),
        recommendation: Finding::recommendation_for_rule_id("security.env-file-committed"),
        title: "Environment file tracked in repository".to_string(),
        description,
        category: FindingCategory::Security,
        severity: Severity::Critical,
        confidence,
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
