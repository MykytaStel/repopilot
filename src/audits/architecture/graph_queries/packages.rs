//! Opt-in `architecture.package-boundary-violation`: a file reaching into
//! another package's internals instead of going through its public API. A
//! "package" is an immediate child of a declared root glob (`packages/*`), or a
//! declared directory itself. With no `[architecture] package_roots` configured
//! the rule is silent.
//!
//! This is the lightweight, path-based form. Promoting workspace packages to
//! first-class graph nodes is tracked separately (roadmap PR-F).

use crate::findings::types::{Evidence, Finding};
use crate::scan::config::ScanConfig;

use super::{NodeInfo, architecture_finding};

pub(crate) struct PackageIndex {
    roots: Vec<String>,
}

impl PackageIndex {
    pub(crate) fn from_config(config: &ScanConfig) -> Self {
        Self {
            roots: config.package_roots.clone(),
        }
    }

    /// The package a file belongs to, or `None` if it is outside every declared
    /// root. `packages/*` maps `packages/auth/src/x.ts` to `packages/auth`; a
    /// bare `libs/core` maps anything under it to `libs/core`.
    fn package_of(&self, info: &NodeInfo) -> Option<String> {
        let rel = info.relative.to_string_lossy().replace('\\', "/");
        for pattern in &self.roots {
            if let Some(base) = pattern.strip_suffix("/*") {
                let prefix = format!("{base}/");
                if let Some(rest) = rel.strip_prefix(&prefix) {
                    let first = rest.split('/').next().unwrap_or("");
                    if !first.is_empty() {
                        return Some(format!("{base}/{first}"));
                    }
                }
            } else if rel == *pattern || rel.starts_with(&format!("{pattern}/")) {
                return Some(pattern.clone());
            }
        }
        None
    }

    pub(crate) fn violation_finding(
        &self,
        source: &NodeInfo,
        target: &NodeInfo,
        root: &std::path::Path,
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

        let (line_start, line_end) = if let Some(facts) = source.facts {
            super::edge_evidence(facts, &target.relative, root, known_files)
        } else {
            (1, None)
        };

        Some(architecture_finding(
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
        ))
    }
}
