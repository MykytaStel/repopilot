//! Opt-in `architecture.layer-violation`: imports that point against the
//! user-declared layer order. Layers are listed highest-level first; a module
//! may depend on layers at or below its own, never above. With no
//! `[[architecture.layers]]` configured the index is empty and the rule is
//! silent.

use globset::{Glob, GlobSet, GlobSetBuilder};

use crate::findings::types::{Evidence, Finding};
use crate::scan::config::ScanConfig;

use super::{NodeInfo, architecture_finding};

pub(crate) struct LayerIndex {
    layers: Vec<(String, GlobSet)>,
}

impl LayerIndex {
    pub(crate) fn from_config(config: &ScanConfig) -> Self {
        let mut layers = Vec::new();
        for layer in &config.architecture_layers {
            let mut builder = GlobSetBuilder::new();
            for pattern in &layer.paths {
                if let Ok(glob) = Glob::new(pattern) {
                    builder.add(glob);
                }
            }
            if let Ok(set) = builder.build() {
                layers.push((layer.name.clone(), set));
            }
        }
        Self { layers }
    }

    /// Position of the file in the declared layer order (0 = highest-level), or
    /// `None` when the file matches no declared layer.
    fn layer_of(&self, info: &NodeInfo) -> Option<usize> {
        self.layers
            .iter()
            .position(|(_, set)| set.is_match(&info.relative))
    }

    pub(crate) fn violation_finding(
        &self,
        source: &NodeInfo,
        target: &NodeInfo,
        root: &std::path::Path,
        known_files: &std::collections::HashSet<std::path::PathBuf>,
    ) -> Option<Finding> {
        if self.layers.is_empty() {
            return None;
        }
        let source_idx = self.layer_of(source)?;
        let target_idx = self.layer_of(target)?;

        // A lower layer (higher index) importing a higher layer (lower index)
        // reverses the declared dependency direction.
        if target_idx >= source_idx {
            return None;
        }

        let source_layer = &self.layers[source_idx].0;
        let target_layer = &self.layers[target_idx].0;

        let (line_start, line_end) = if let Some(facts) = source.facts {
            super::edge_evidence(facts, &target.relative, root, known_files)
        } else {
            (1, None)
        };

        Some(architecture_finding(
            "architecture.layer-violation",
            "Layer violation detected",
            format!(
                "The `{source_layer}` layer imports the higher-level `{target_layer}` layer, against the declared layer order.",
            ),
            Evidence {
                path: source.relative.clone(),
                line_start,
                line_end,
                snippet: format!(
                    "{source_layer} → {target_layer}: imports {}",
                    target.relative.display()
                ),
            },
        ))
    }
}
