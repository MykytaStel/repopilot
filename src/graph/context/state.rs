use super::{RepoContextGraph, RepoContextNode};
use crate::frameworks::{DetectedFramework, FrameworkProject, ReactNativeArchitectureProfile};
use crate::graph::CouplingGraph;
use crate::review::diff::ChangedFile;
use crate::scan::facts::{FileFacts, ScanFacts};
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub(crate) struct RepositoryContextState {
    root_path: PathBuf,
    files: Vec<RepoContextNode>,
    coupling: CouplingGraph,
    detected_frameworks: Vec<DetectedFramework>,
    framework_projects: Vec<FrameworkProject>,
    react_native: Option<ReactNativeArchitectureProfile>,
}

impl RepositoryContextState {
    pub(crate) fn from_scan_facts(facts: &ScanFacts, root: &Path, coupling: CouplingGraph) -> Self {
        Self::from_compat(RepoContextGraph::from_scan_facts(facts, root, coupling))
    }

    pub(crate) fn from_compat(graph: RepoContextGraph) -> Self {
        let mut files = graph.nodes;
        files.sort_by(|left, right| left.path.cmp(&right.path));
        let nodes = files.iter().map(|file| file.path.clone()).collect();
        Self {
            root_path: graph.root_path,
            files,
            coupling: CouplingGraph {
                edges: graph.edges,
                deferred_edges: graph.deferred_edges,
                nodes,
            },
            detected_frameworks: graph.detected_frameworks,
            framework_projects: graph.framework_projects,
            react_native: graph.react_native,
        }
    }

    pub(crate) fn to_compat(&self) -> RepoContextGraph {
        RepoContextGraph {
            root_path: self.root_path.clone(),
            nodes: self.files.clone(),
            edges: self.coupling.edges.clone(),
            deferred_edges: self.coupling.deferred_edges.clone(),
            detected_frameworks: self.detected_frameworks.clone(),
            framework_projects: self.framework_projects.clone(),
            react_native: self.react_native.clone(),
        }
    }

    pub(crate) fn coupling_graph(&self) -> CouplingGraph {
        self.coupling.clone()
    }

    pub(crate) fn to_scan_facts(&self) -> ScanFacts {
        self.to_compat().to_scan_facts()
    }

    pub(crate) fn apply_changed_facts(
        &mut self,
        root: &Path,
        changed_files: &[ChangedFile],
        patch_files: &[FileFacts],
    ) {
        let mut graph = self.to_compat();
        graph.apply_changed_facts(root, changed_files, patch_files);
        *self = Self::from_compat(graph);
    }
}
