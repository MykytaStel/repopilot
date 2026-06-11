use std::path::{Path, PathBuf};

use super::high_instability_hub_finding;
use crate::findings::types::Confidence;
use crate::graph::{FileMetrics, ImportResolutionStats};

fn hub_metric() -> FileMetrics {
    FileMetrics {
        path: PathBuf::from("src/hub.ts"),
        fan_in: 6,
        fan_out: 14,
        instability: 0.7,
    }
}

#[test]
fn hub_confidence_is_high_when_graph_is_complete() {
    let finding = high_instability_hub_finding(
        &hub_metric(),
        Path::new(""),
        70,
        5,
        70,
        None,
        &ImportResolutionStats::default(),
    );

    assert_eq!(finding.confidence, Confidence::High);
    assert!(
        !finding.evidence[0].snippet.contains("unresolved"),
        "no demotion note expected for a complete graph"
    );
}

#[test]
fn hub_confidence_is_demoted_when_unresolved_imports_exist() {
    let mut resolution = ImportResolutionStats::default();
    resolution.record(Path::new("src/a.ts"), "./missing");

    let finding =
        high_instability_hub_finding(&hub_metric(), Path::new(""), 70, 5, 70, None, &resolution);

    assert_eq!(finding.confidence, Confidence::Medium);
    assert!(
        finding.evidence[0]
            .snippet
            .contains("fan-in may be undercounted"),
        "snippet should explain the demotion: {}",
        finding.evidence[0].snippet
    );
}
