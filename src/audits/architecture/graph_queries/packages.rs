//! `architecture.package-boundary-violation`: a file reaching into another
//! package's internals instead of going through its public API.
//!
//! Packages come from one of two sources, in priority order:
//!
//! 1. **Explicit** `[architecture] package_roots` globs (`packages/*`, or a bare
//!    directory). Findings carry the registry-default confidence.
//! 2. **Detected** npm/pnpm/Cargo/Go workspaces, used when no `package_roots`
//!    are configured. The rule auto-enables on a workspace, and findings are
//!    raised to `High` confidence because the boundary is declared by the
//!    repository's own manifests.
//!
//! With neither source the rule is silent.

use std::collections::HashSet;
use std::path::Path;

use crate::findings::types::{Confidence, Evidence, Finding};
use crate::scan::config::ScanConfig;
use crate::scan::workspace::WorkspacePackage;

use super::{NodeInfo, architecture_finding};

pub(crate) struct PackageIndex {
    /// Either glob patterns (`packages/*`) or bare package roots (`packages/auth`).
    roots: Vec<String>,
    /// Relative roots of detected packages whose `package.json` `exports` map
    /// declares a wildcard subpath — deep imports into them are sanctioned
    /// public API, so they have no boundary to violate.
    open_export_roots: HashSet<String>,
    /// True when `roots` were derived from workspace manifests rather than
    /// configured globs; such findings are reported at `High` confidence.
    manifest_backed: bool,
}

impl PackageIndex {
    /// Configured `package_roots` win (preserving the explicit, pre-0.18
    /// behavior); otherwise detected workspace packages drive the rule.
    pub(crate) fn new(
        config: &ScanConfig,
        detected: &[WorkspacePackage],
        repo_root: &Path,
    ) -> Self {
        if !config.package_roots.is_empty() {
            return Self {
                roots: config.package_roots.clone(),
                open_export_roots: HashSet::new(),
                manifest_backed: false,
            };
        }

        let mut roots = Vec::new();
        let mut open_export_roots = HashSet::new();
        for package in detected {
            let Ok(relative) = package.root.strip_prefix(repo_root) else {
                continue;
            };
            let relative = relative.to_string_lossy().replace('\\', "/");
            if relative.is_empty() {
                continue;
            }
            if package.exposes_subpath_exports {
                open_export_roots.insert(relative.clone());
            }
            roots.push(relative);
        }

        Self {
            roots,
            open_export_roots,
            manifest_backed: true,
        }
    }

    /// The package a file belongs to, or `None` if it is outside every root.
    /// `packages/*` maps `packages/auth/src/x.ts` to `packages/auth`; a bare
    /// `libs/core` maps anything under it to `libs/core`. The most specific
    /// (longest) match wins so nested packages are attributed correctly.
    fn package_of(&self, info: &NodeInfo) -> Option<String> {
        let rel = info.relative.to_string_lossy().replace('\\', "/");
        let mut best: Option<String> = None;
        for pattern in &self.roots {
            let candidate = if let Some(base) = pattern.strip_suffix("/*") {
                let prefix = format!("{base}/");
                rel.strip_prefix(&prefix)
                    .and_then(|rest| rest.split('/').next())
                    .filter(|first| !first.is_empty())
                    .map(|first| format!("{base}/{first}"))
            } else if rel == *pattern || rel.starts_with(&format!("{pattern}/")) {
                Some(pattern.clone())
            } else {
                None
            };

            if let Some(candidate) = candidate
                && best
                    .as_ref()
                    .is_none_or(|current| candidate.len() > current.len())
            {
                best = Some(candidate);
            }
        }
        best
    }

    pub(crate) fn violation_finding(
        &self,
        source: &NodeInfo,
        target: &NodeInfo,
        root: &Path,
        known_files: &std::collections::HashSet<std::path::PathBuf>,
    ) -> Option<Finding> {
        if self.roots.is_empty() || target.context.is_public_api {
            return None;
        }
        let source_pkg = self.package_of(source)?;
        let target_pkg = self.package_of(target)?;
        if source_pkg == target_pkg {
            return None;
        }
        // A package that publishes a wildcard `exports` subpath has declared its
        // internals public — deep imports into it are sanctioned, not violations.
        if self.open_export_roots.contains(&target_pkg) {
            return None;
        }

        let (line_start, line_end) = if let Some(facts) = source.facts {
            super::edge_evidence(facts, &target.relative, root, known_files)
        } else {
            (1, None)
        };

        let mut finding = architecture_finding(
            "architecture.package-boundary-violation",
            "Package boundary violation",
            format!(
                "`{source_pkg}` imports a private module of `{target_pkg}` instead of its public API.",
            ),
            Evidence {
                path: source.relative.clone(),
                line_start,
                line_end,
                snippet: format!("imports internal file: {}", target.relative.display()),
            },
        );

        // A manifest-declared boundary is a fact about the repository, not a
        // heuristic guess, so it earns the registry's confidence ceiling.
        if self.manifest_backed {
            finding.confidence = Confidence::High;
        }
        Some(finding)
    }

    #[cfg(test)]
    pub(crate) fn from_config(config: &ScanConfig) -> Self {
        Self::new(config, &[], Path::new(""))
    }
}
