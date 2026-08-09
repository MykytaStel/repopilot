use crate::graph::v2::{GraphNodeId, GraphSnapshot};
use crate::graph::{CouplingGraph, ImportResolutionStats};
use crate::scan::config::ScanConfig;
use crate::scan::facts::ScanFacts;
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

pub(crate) struct GraphAuditContext<'a> {
    pub(crate) facts: &'a ScanFacts,
    pub(crate) config: &'a ScanConfig,
    pub(crate) root: &'a Path,
    pub(crate) graph: &'a CouplingGraph,
    pub(crate) resolution: &'a ImportResolutionStats,
    pub(crate) snapshot: &'a GraphSnapshot,
    pub(crate) path_by_id: &'a BTreeMap<GraphNodeId, PathBuf>,
}
