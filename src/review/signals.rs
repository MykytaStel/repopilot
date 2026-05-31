//! Security-boundary change signals for `repopilot review`.
//!
//! These are review-layer signals, not scan-engine rules: they answer a single
//! question from the diff's changed-file list — *did this change touch a part of
//! the repo that decides who can do what, or how the app ships?*
//!
//! They **flag, they do not prove.** A false positive costs a glance, so the
//! defaults lean toward surfacing rather than staying silent. This is the
//! opposite trade-off from a security scanner, where every false alarm is waste.
//! The detector is purely path/filename classification over `changed_files`;
//! it never inspects code semantics and never claims a change is safe or unsafe.
//! Ships at `preview` — the default pattern set will need tuning from real repos.

use crate::config::model::SecurityBoundarySection;
use crate::review::diff::{ChangeStatus, ChangedFile};
use globset::{Glob, GlobSet, GlobSetBuilder};
use serde::Serialize;
use std::collections::HashSet;

/// The kind of security boundary a changed file belongs to.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum BoundaryCategory {
    /// Who can do what: auth, sessions, permissions, RBAC, identity.
    AccessControl,
    /// Whether the boundary trusts a request: CORS, CSP, security headers.
    RequestTrust,
    /// How the app ships: CI/workflows, Dockerfiles, IaC, deploy config.
    DeploySurface,
    /// What code runs: dependency manifests and lockfiles.
    SupplyChain,
    /// Where secrets/config live: committed `.env` files.
    SecretConfig,
    /// A user-supplied `extra_patterns` match from `repopilot.toml`.
    Custom,
}

impl BoundaryCategory {
    /// Human-readable label used in console/markdown output.
    pub fn label(self) -> &'static str {
        match self {
            Self::AccessControl => "access control",
            Self::RequestTrust => "request trust",
            Self::DeploySurface => "deploy surface",
            Self::SupplyChain => "supply chain",
            Self::SecretConfig => "secret config",
            Self::Custom => "custom",
        }
    }
}

/// One changed file that crossed a security boundary.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct BoundarySignal {
    pub category: BoundaryCategory,
    pub path: String,
    pub status: ChangeStatus,
}

/// Classify each changed file against the security-boundary categories.
///
/// Returns at most one signal per changed file (the first category it matches,
/// in priority order). Sorted by category then path for deterministic output.
pub fn detect_boundary_signals(
    changed_files: &[ChangedFile],
    config: &SecurityBoundarySection,
) -> Vec<BoundarySignal> {
    if !config.enabled {
        return Vec::new();
    }

    let custom = build_custom_globset(&config.extra_patterns);

    let mut signals: Vec<BoundarySignal> = changed_files
        .iter()
        .filter_map(|file| {
            let path = file.path_string();
            classify_boundary(&path, custom.as_ref()).map(|category| BoundarySignal {
                category,
                path,
                status: file.status,
            })
        })
        .collect();

    signals.sort_by(|left, right| {
        left.category
            .cmp(&right.category)
            .then_with(|| left.path.cmp(&right.path))
    });
    signals
}

fn classify_boundary(path: &str, custom: Option<&GlobSet>) -> Option<BoundaryCategory> {
    let lower = path.to_ascii_lowercase();
    let file_name = lower.rsplit('/').next().unwrap_or(lower.as_str());

    // Priority order: the more specific / path-anchored categories win so a
    // workflow file named `auth.yml` is reported as deploy surface, not access
    // control.
    if is_deploy_surface(&lower, file_name) {
        return Some(BoundaryCategory::DeploySurface);
    }
    if is_supply_chain(file_name) {
        return Some(BoundaryCategory::SupplyChain);
    }
    if is_secret_config(file_name) {
        return Some(BoundaryCategory::SecretConfig);
    }

    let tokens = tokenize(path);
    if is_request_trust(&tokens) {
        return Some(BoundaryCategory::RequestTrust);
    }
    if is_access_control(&lower, &tokens) {
        return Some(BoundaryCategory::AccessControl);
    }

    if let Some(set) = custom
        && set.is_match(path)
    {
        return Some(BoundaryCategory::Custom);
    }

    None
}

fn is_deploy_surface(lower: &str, file_name: &str) -> bool {
    lower.contains(".github/workflows/")
        || lower.contains("/.circleci/")
        || lower.starts_with(".circleci/")
        || lower.ends_with(".tf")
        || lower.ends_with(".tfvars")
        || file_name == "containerfile"
        || file_name == "jenkinsfile"
        || file_name == "action.yml"
        || file_name == "action.yaml"
        || file_name == ".gitlab-ci.yml"
        || file_name == "azure-pipelines.yml"
        || file_name == "azure-pipelines.yaml"
        || file_name == "dockerfile"
        || file_name.starts_with("dockerfile.")
}

fn is_supply_chain(file_name: &str) -> bool {
    matches!(
        file_name,
        "package.json"
            | "package-lock.json"
            | "yarn.lock"
            | "pnpm-lock.yaml"
            | "npm-shrinkwrap.json"
            | "cargo.toml"
            | "cargo.lock"
            | "go.mod"
            | "go.sum"
            | "requirements.txt"
            | "pyproject.toml"
            | "poetry.lock"
            | "pipfile"
            | "pipfile.lock"
            | "pom.xml"
            | "build.gradle"
            | "build.gradle.kts"
            | "gemfile"
            | "gemfile.lock"
    )
}

fn is_secret_config(file_name: &str) -> bool {
    file_name == ".env" || file_name.starts_with(".env.")
}

fn is_request_trust(tokens: &HashSet<String>) -> bool {
    tokens.contains("cors") || tokens.contains("csp") || tokens.contains("helmet")
}

fn is_access_control(lower: &str, tokens: &HashSet<String>) -> bool {
    // High-specificity substrings: collisions in real paths are negligible.
    // `authentic` covers authenticate/authentication/authenticator; `authoriz`
    // covers authorize/authorization/authorized (but not the vaguer `authority`).
    const STRONG: &[&str] = &["oauth", "jwt", "rbac", "passport", "authentic", "authoriz"];
    if STRONG.iter().any(|needle| lower.contains(needle)) {
        return true;
    }

    // Token-equality (after camelCase + separator splitting) keeps `auth` from
    // matching `authors`, and `session` from matching `sessionize`, etc. Design
    // tokens (`token`, `policy`) are deliberately excluded to avoid noisy false
    // positives in front-end repos; users can add them via `extra_patterns`.
    const TOKENS: &[&str] = &[
        "auth",
        "authz",
        "authn",
        "login",
        "logout",
        "signin",
        "signout",
        "session",
        "sessions",
        "permission",
        "permissions",
        "acl",
        "guard",
        "guards",
        "identity",
        "credential",
        "credentials",
    ];
    TOKENS.iter().any(|token| tokens.contains(*token))
}

/// Split a path into lowercase alphanumeric tokens, breaking on separators and
/// on lowercase→uppercase transitions so `authMiddleware.ts` yields `auth`.
fn tokenize(text: &str) -> HashSet<String> {
    let mut tokens = HashSet::new();
    let mut current = String::new();

    for ch in text.chars() {
        if !ch.is_ascii_alphanumeric() {
            flush(&mut current, &mut tokens);
            continue;
        }
        if ch.is_ascii_uppercase()
            && current
                .chars()
                .last()
                .is_some_and(|last| last.is_ascii_lowercase() || last.is_ascii_digit())
        {
            flush(&mut current, &mut tokens);
        }
        current.push(ch.to_ascii_lowercase());
    }
    flush(&mut current, &mut tokens);
    tokens
}

fn flush(current: &mut String, tokens: &mut HashSet<String>) {
    if !current.is_empty() {
        tokens.insert(std::mem::take(current));
    }
}

fn build_custom_globset(patterns: &[String]) -> Option<GlobSet> {
    if patterns.is_empty() {
        return None;
    }

    let mut builder = GlobSetBuilder::new();
    let mut added = false;
    for pattern in patterns {
        if let Ok(glob) = Glob::new(pattern) {
            builder.add(glob);
            added = true;
        }
    }

    if !added {
        return None;
    }
    builder.build().ok()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::review::diff::{ChangeStatus, ChangedFile};
    use std::path::PathBuf;

    fn changed(path: &str, status: ChangeStatus) -> ChangedFile {
        ChangedFile {
            path: PathBuf::from(path),
            status,
            ranges: Vec::new(),
        }
    }

    fn classify(path: &str) -> Option<BoundaryCategory> {
        classify_boundary(path, None)
    }

    #[test]
    fn access_control_matches_auth_paths_and_filenames() {
        assert_eq!(
            classify("src/middleware/auth.ts"),
            Some(BoundaryCategory::AccessControl)
        );
        assert_eq!(
            classify("src/authMiddleware.ts"),
            Some(BoundaryCategory::AccessControl)
        );
        assert_eq!(
            classify("internal/session/store.go"),
            Some(BoundaryCategory::AccessControl)
        );
        assert_eq!(
            classify("src/auth/jwt_verify.rs"),
            Some(BoundaryCategory::AccessControl)
        );
        assert_eq!(
            classify("app/Http/Middleware/Authenticate.php"),
            Some(BoundaryCategory::AccessControl)
        );
    }

    #[test]
    fn access_control_does_not_match_lookalike_tokens() {
        // `authors`, design `tokens`, and privacy `policy` are common non-boundary files.
        assert_eq!(classify("src/components/authors.tsx"), None);
        assert_eq!(classify("src/theme/tokens.ts"), None);
        assert_eq!(classify("docs/privacy-policy.md"), None);
        assert_eq!(classify("src/utils/format.ts"), None);
    }

    #[test]
    fn request_trust_matches_cors_and_helmet() {
        assert_eq!(
            classify("src/server/cors.ts"),
            Some(BoundaryCategory::RequestTrust)
        );
        assert_eq!(
            classify("config/helmet.js"),
            Some(BoundaryCategory::RequestTrust)
        );
        assert_eq!(
            classify("src/corsConfig.ts"),
            Some(BoundaryCategory::RequestTrust)
        );
    }

    #[test]
    fn deploy_surface_matches_ci_docker_and_iac() {
        assert_eq!(
            classify(".github/workflows/deploy.yml"),
            Some(BoundaryCategory::DeploySurface)
        );
        assert_eq!(
            classify("Dockerfile"),
            Some(BoundaryCategory::DeploySurface)
        );
        assert_eq!(
            classify("Dockerfile.prod"),
            Some(BoundaryCategory::DeploySurface)
        );
        assert_eq!(
            classify("infra/main.tf"),
            Some(BoundaryCategory::DeploySurface)
        );
        assert_eq!(
            classify(".gitlab-ci.yml"),
            Some(BoundaryCategory::DeploySurface)
        );
    }

    #[test]
    fn deploy_surface_wins_over_access_control_for_workflow_named_auth() {
        assert_eq!(
            classify(".github/workflows/auth.yml"),
            Some(BoundaryCategory::DeploySurface)
        );
    }

    #[test]
    fn supply_chain_matches_manifests_and_lockfiles() {
        assert_eq!(
            classify("package.json"),
            Some(BoundaryCategory::SupplyChain)
        );
        assert_eq!(
            classify("frontend/package-lock.json"),
            Some(BoundaryCategory::SupplyChain)
        );
        assert_eq!(classify("Cargo.toml"), Some(BoundaryCategory::SupplyChain));
        assert_eq!(classify("go.sum"), Some(BoundaryCategory::SupplyChain));
    }

    #[test]
    fn secret_config_matches_env_files() {
        assert_eq!(classify(".env"), Some(BoundaryCategory::SecretConfig));
        assert_eq!(
            classify(".env.production"),
            Some(BoundaryCategory::SecretConfig)
        );
        assert_eq!(classify("src/config/settings.ts"), None);
    }

    #[test]
    fn disabled_config_yields_no_signals() {
        let config = SecurityBoundarySection {
            enabled: false,
            extra_patterns: Vec::new(),
        };
        let files = vec![changed("src/auth/login.ts", ChangeStatus::Modified)];
        assert!(detect_boundary_signals(&files, &config).is_empty());
    }

    #[test]
    fn extra_patterns_flag_custom_boundaries() {
        let config = SecurityBoundarySection {
            enabled: true,
            extra_patterns: vec!["**/secrets/**".to_string()],
        };
        let files = vec![
            changed("ops/secrets/keys.yaml", ChangeStatus::Added),
            changed("src/utils/format.ts", ChangeStatus::Modified),
        ];
        let signals = detect_boundary_signals(&files, &config);
        assert_eq!(signals.len(), 1);
        assert_eq!(signals[0].category, BoundaryCategory::Custom);
        assert_eq!(signals[0].path, "ops/secrets/keys.yaml");
    }

    #[test]
    fn detect_sorts_by_category_then_path() {
        let config = SecurityBoundarySection::default();
        let files = vec![
            changed("package.json", ChangeStatus::Modified),
            changed("src/auth/login.ts", ChangeStatus::Modified),
            changed(".github/workflows/ci.yml", ChangeStatus::Modified),
        ];
        let signals = detect_boundary_signals(&files, &config);
        let categories: Vec<_> = signals.iter().map(|signal| signal.category).collect();
        assert_eq!(
            categories,
            vec![
                BoundaryCategory::AccessControl,
                BoundaryCategory::DeploySurface,
                BoundaryCategory::SupplyChain,
            ]
        );
    }
}
