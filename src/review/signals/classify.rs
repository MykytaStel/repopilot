//! Path/filename classification for security-boundary signals.
//!
//! Pure functions: given a repo-relative path, decide which [`BoundaryCategory`]
//! (if any) it belongs to. No file contents are read. See the module docs in
//! `mod.rs` for the "flag, don't prove" philosophy.

use super::BoundaryCategory;
use globset::{Glob, GlobSet, GlobSetBuilder};
use std::collections::HashSet;

/// Classify a path into a boundary category, or `None` if it crosses none.
///
/// Priority order matters: more specific / path-anchored categories win, so a
/// workflow file named `auth.yml` is reported as deploy surface, not access
/// control.
pub(super) fn classify_boundary(path: &str, custom: Option<&GlobSet>) -> Option<BoundaryCategory> {
    let lower = path.to_ascii_lowercase();
    let file_name = lower.rsplit('/').next().unwrap_or(lower.as_str());

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

/// Compile the user's `extra_patterns` into a glob set, skipping any invalid
/// pattern. Returns `None` when there is nothing to match.
pub(super) fn build_custom_globset(patterns: &[String]) -> Option<GlobSet> {
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
