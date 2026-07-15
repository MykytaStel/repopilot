//! Path/filename classification for security-boundary signals.
//!
//! Pure functions: given a repo-relative path, decide which [`BoundaryCategory`]
//! (if any) it belongs to. No file contents are read. See the module docs in
//! `mod.rs` for the "flag, don't prove" philosophy.

use super::BoundaryCategory;
#[cfg(test)]
use crate::analysis::parse::ParsedFile;
use crate::review::signals::content::ReviewSource;
use globset::{Glob, GlobSet, GlobSetBuilder};
use std::collections::HashSet;
#[cfg(test)]
use std::path::Path;

/// Classify a path into a boundary category, or `None` if it crosses none.
///
/// Priority order matters: more specific / path-anchored categories win, so a
/// workflow file named `auth.yml` is reported as deploy surface, not access
/// control.
pub(in crate::review) fn classify_boundary(
    path: &str,
    custom: Option<&GlobSet>,
) -> Option<BoundaryCategory> {
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
pub(in crate::review) fn build_custom_globset(patterns: &[String]) -> Option<GlobSet> {
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

/// Classify the syntax tree of a modified file using AST queries.
///
/// Scans the file contents for security-relevant decorators, annotations,
/// macros, and specific library imports to detect access control or request trust boundaries.
///
/// Test-only: exercises `walk_ast_for_boundary` over raw `(path, content)` in
/// isolation. Production code shares a parse via
/// [`classify_boundary_ast_from_source`] instead.
#[cfg(test)]
pub(super) fn classify_boundary_ast(path: &Path, content: &str) -> Option<BoundaryCategory> {
    let language_label = crate::scan::language::detect_language(path)?;
    let parsed = ParsedFile::new(content, Some(language_label));
    let tree = parsed.tree()?;
    walk_ast_for_boundary(tree.root_node(), content, language_label)
}

/// Same classification, but over an already-parsed [`ReviewSource`] — shares
/// its memoized tree instead of parsing the post-change content a second
/// time. Used by the production review pass, which already has a source in
/// hand; [`classify_boundary_ast`] stays for isolated unit tests.
pub(in crate::review) fn classify_boundary_ast_from_source(
    source: &ReviewSource,
) -> Option<BoundaryCategory> {
    let language_label = source.language_label()?;
    let tree = source.tree()?;
    walk_ast_for_boundary(tree.root_node(), source.content(), language_label)
}

fn walk_ast_for_boundary(
    node: tree_sitter::Node<'_>,
    content: &str,
    language_label: &str,
) -> Option<BoundaryCategory> {
    if let Some(category) = match_node_for_boundary(node, content, language_label) {
        return Some(category);
    }

    let mut cursor = node.walk();
    for child in node.children(&mut cursor) {
        if let Some(category) = walk_ast_for_boundary(child, content, language_label) {
            return Some(category);
        }
    }
    None
}

/// Vague access-control words matched as **whole tokens** (after the same
/// camelCase + separator splitting `classify_boundary` uses), so `author` does
/// not read as `auth`, `roleplay` as `role`, `sessionStorage` as `session`, or
/// `securityLogger` as a boundary. Applied to decorators/annotations/attributes
/// and to import statements alike.
const ACCESS_TOKENS: &[&str] = &[
    "auth",
    "authz",
    "authn",
    "login",
    "logout",
    "signin",
    "signout",
    "role",
    "roles",
    "permission",
    "permissions",
    "guard",
    "guards",
    "rbac",
    "acl",
    "jwt",
    "jose",
];

/// High-specificity access-control needles safe to match as **substrings**: auth
/// scheme stems and library/namespace names that do not collide with ordinary
/// identifiers. `authentic` covers authenticate/authentication and `authoriz`
/// covers authorize/authorization without matching the vaguer `authority`.
const ACCESS_STRONG: &[&str] = &[
    "oauth",
    "authentic",
    "authoriz",
    "passport",
    "jsonwebtoken",
    "bcrypt",
    "argon2",
    "pwhash",
    "passlib",
    "authlib",
    "flask_login",
    "flask_jwt_extended",
    "express-session",
    "cookie-session",
    "jwt-go",
    "casbin",
    "authboss",
    "springframework.security",
    "springsecurity",
    "javax.security",
    "jakarta.security",
    "aspnetcore.authentication",
    "aspnetcore.authorization",
    "aspnetcore.identity",
    "system.security",
    "identitymodel",
    "identityserver",
    "cryptography",
];

/// Request-trust terms (CORS / security headers), matched as whole tokens.
const REQUEST_TRUST_TOKENS: &[&str] = &["cors", "csp", "helmet"];

/// Whether `text` (already lowercased) names a boundary concept: any
/// high-specificity `strong` substring, or any vague `tokens` term as a whole
/// token. The token path is what keeps `author`/`sessionStorage`/`security_logger`
/// from being misread as boundaries — the same fix applied to the coarse
/// behavioral fallback.
fn text_names_boundary(text: &str, strong: &[&str], tokens: &[&str]) -> bool {
    if strong.iter().any(|needle| text.contains(needle)) {
        return true;
    }
    if tokens.is_empty() {
        return false;
    }
    let found = tokenize(text);
    tokens.iter().any(|token| found.contains(*token))
}

fn match_node_for_boundary(
    node: tree_sitter::Node<'_>,
    content: &str,
    language: &str,
) -> Option<BoundaryCategory> {
    // Which node kinds carry a boundary signal comes from the language
    // frontend's boundary table; the matching itself is shared and
    // token-aware. `check_request_trust` is only set for import-like nodes,
    // where a CORS/header library can appear.
    let kinds = crate::languages::review_for_label(language)?.boundary?;
    let check_request_trust = if kinds.decorator_kinds.contains(&node.kind()) {
        false
    } else if kinds.import_kinds.contains(&node.kind()) {
        true
    } else {
        return None;
    };

    let text = node
        .utf8_text(content.as_bytes())
        .ok()?
        .to_ascii_lowercase();

    if text_names_boundary(&text, ACCESS_STRONG, ACCESS_TOKENS) {
        return Some(BoundaryCategory::AccessControl);
    }
    if check_request_trust && text_names_boundary(&text, &[], REQUEST_TRUST_TOKENS) {
        return Some(BoundaryCategory::RequestTrust);
    }
    None
}
