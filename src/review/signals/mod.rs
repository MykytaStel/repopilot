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
//!
//! - [`classify`] maps a single path to a [`BoundaryCategory`].
//! - [`composites`] enriches signals with review context already on hand: the
//!   blast radius (how far the changed file reaches) and whether the change
//!   touched a code boundary without touching any test.

pub mod algorithmic;
#[cfg(test)]
mod algorithmic_tests;
pub mod behavioral;
#[cfg(test)]
mod behavioral_tests;
mod classify;
pub mod composites;
pub mod content;
pub mod taint;
#[cfg(test)]
mod tests;
pub mod tiered;
#[cfg(test)]
mod tiered_tests;

use crate::audits::context::classify::helpers::is_test_file;
use crate::config::model::SecurityBoundarySection;
use crate::review::diff::{ChangeStatus, ChangedFile, DiffTarget};
use serde::Serialize;
use std::path::Path;

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

    /// Whether this boundary is application *code* (where a missing test change
    /// is meaningful), as opposed to config/manifest/deploy surfaces that rarely
    /// have unit tests. Drives the "boundary changed, no test touched" signal.
    pub fn is_code_boundary(self) -> bool {
        matches!(self, Self::AccessControl | Self::RequestTrust)
    }
}

/// One changed file that crossed a security boundary.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct BoundarySignal {
    pub category: BoundaryCategory,
    pub path: String,
    pub status: ChangeStatus,
    /// Number of other files that import this changed file (review composite).
    /// `0` when nothing imports it or no import graph is available. Populated
    /// after detection by [`composites::enrich_blast_radius`].
    pub blast_radius: usize,
}

/// Classify each changed file against the security-boundary categories.
///
/// Returns at most one signal per changed file (the first category it matches,
/// in priority order). Sorted by category then path for deterministic output.
/// `blast_radius` is left at `0` here; callers enrich it via [`composites`].
///
/// Test files are skipped: a boundary signal means "the change touched code that
/// decides who-can-do-what or how the app ships," and a test for that boundary
/// decides neither. Without this, `auth_service_test.ts` would land in the
/// top confidence tier as "access control changed." Whether a test *moved
/// alongside* a code-boundary change is a separate signal — see
/// [`composites::missing_test_for_code_boundary`].
pub fn detect_boundary_signals(
    repo_root: &Path,
    target: DiffTarget<'_>,
    changed_files: &[ChangedFile],
    config: &SecurityBoundarySection,
) -> Vec<BoundarySignal> {
    if !config.enabled {
        return Vec::new();
    }

    let custom = classify::build_custom_globset(&config.extra_patterns);

    let mut signals: Vec<BoundarySignal> = changed_files
        .iter()
        .filter(|file| !is_test_file(&file.path, false))
        .filter_map(|file| {
            let path = file.path_string();
            let mut category = classify::classify_boundary(&path, custom.as_ref());

            if category.is_none() {
                if let Some(post_source) = content::post_change_source(repo_root, file, target) {
                    category = classify::classify_boundary_ast(&file.path, post_source.content());
                }
            }

            category.map(|category| BoundarySignal {
                category,
                path,
                status: file.status,
                blast_radius: 0,
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
