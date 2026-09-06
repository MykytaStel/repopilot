use super::OwnershipIndex;
use crate::review::diff::ChangedFile;
use crate::review::impact::ImpactPaths;
use serde::Serialize;
use std::collections::BTreeSet;
use std::path::PathBuf;

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize)]
pub struct Owner {
    pub value: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct OwnershipDiagnostic {
    pub message: String,
    pub line: Option<usize>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct PathOwnership {
    pub path: String,
    pub owners: Vec<Owner>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub fallback_boundary: Option<String>,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum OwnershipAssessment {
    Resolved,
    ConfiguredButUnmatched,
    #[default]
    NotConfigured,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize)]
pub struct OwnershipSummary {
    #[serde(default)]
    pub assessment: OwnershipAssessment,
    pub paths: Vec<PathOwnership>,
    pub suggested_owners: Vec<Owner>,
    pub unowned_paths: Vec<String>,
    pub fallback_boundaries: Vec<String>,
}

impl OwnershipSummary {
    pub fn for_impact(
        changed: &[ChangedFile],
        impact: &ImpactPaths,
        index: &OwnershipIndex,
    ) -> Self {
        let mut paths = changed
            .iter()
            .map(|file| file.path.clone())
            .collect::<BTreeSet<_>>();
        for file in &impact.files {
            paths.insert(file.path.clone());
            paths.extend(file.direct_dependents.iter().cloned());
            paths.extend(file.transitive_dependents.iter().cloned());
        }
        Self::for_paths(paths, index)
    }

    pub fn for_paths(paths: impl IntoIterator<Item = PathBuf>, index: &OwnershipIndex) -> Self {
        let mut normalized = paths
            .into_iter()
            .map(|path| path.to_string_lossy().replace('\\', "/"))
            .collect::<Vec<_>>();
        normalized.sort();
        normalized.dedup();

        let mut summary = Self::default();
        let mut owners = BTreeSet::new();
        let mut boundaries = BTreeSet::new();
        let configured = index.source().is_some();
        for path in normalized {
            let matched = index.owners_for(path.as_ref());
            let boundary = matched.is_empty().then(|| fallback_boundary(&path));
            if configured && matched.is_empty() {
                summary.unowned_paths.push(path.clone());
            }
            if let Some(boundary) = &boundary {
                boundaries.insert(boundary.clone());
            }
            owners.extend(matched.iter().cloned());
            summary.paths.push(PathOwnership {
                path,
                owners: matched,
                fallback_boundary: boundary,
            });
        }
        summary.suggested_owners = owners.into_iter().collect();
        summary.fallback_boundaries = boundaries.into_iter().collect();
        summary.assessment = if !configured {
            OwnershipAssessment::NotConfigured
        } else if summary.unowned_paths.is_empty() {
            OwnershipAssessment::Resolved
        } else {
            OwnershipAssessment::ConfiguredButUnmatched
        };
        summary
    }
}

fn fallback_boundary(path: &str) -> String {
    path.split('/')
        .next()
        .filter(|component| path.contains('/') && !component.is_empty())
        .unwrap_or(".")
        .to_string()
}
