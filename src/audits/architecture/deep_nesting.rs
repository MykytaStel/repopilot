use crate::audits::traits::ProjectAudit;
use crate::findings::types::{Evidence, Finding, FindingCategory, Severity};
use crate::scan::config::ScanConfig;
use crate::scan::facts::ScanFacts;
use std::path::PathBuf;

pub struct DeepNestingAudit;

impl ProjectAudit for DeepNestingAudit {
    fn audit(&self, facts: &ScanFacts, config: &ScanConfig) -> Vec<Finding> {
        let root_depth = facts.root_path.components().count();

        let deepest = facts
            .files
            .iter()
            .filter_map(|file| {
                let depth = file.path.components().count().saturating_sub(root_depth);
                if depth > config.max_directory_depth {
                    Some((file.path.clone(), depth))
                } else {
                    None
                }
            })
            .max_by_key(|(_, depth)| *depth);

        match deepest {
            None => vec![],
            Some((path, depth)) => vec![build_finding(path, depth, config.max_directory_depth)],
        }
    }
}

fn build_finding(deepest_path: PathBuf, depth: usize, threshold: usize) -> Finding {
    Finding {
        id: format!(
            "architecture.deep-nesting.{}",
            deepest_path.display()
        ),
        rule_id: "architecture.deep-nesting".to_string(),
        title: "Deeply nested directory structure detected".to_string(),
        description: format!(
            "The project contains files nested {depth} levels deep, exceeding the threshold of {threshold}. Deep nesting often indicates over-engineered module hierarchies."
        ),
        category: FindingCategory::Architecture,
        severity: Severity::Low,
        evidence: vec![Evidence {
            path: deepest_path,
            line_start: 1,
            line_end: None,
            snippet: format!("Nesting depth: {depth}; threshold is {threshold}."),
        }],
    }
}
