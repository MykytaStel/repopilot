mod base_severity;
pub mod builder;
pub mod finding;
pub mod model;
pub mod render;

pub use base_severity::base_severity_for_explain;
pub use builder::{build_explain_report, build_explain_report_with_root};
pub use finding::{
    FindingAmbiguityReport, FindingDecisionReplay, FindingExplanationReport,
    FindingOccurrenceCandidate, FindingOccurrenceLocator, FindingReplayStatus,
    FindingSelectionReport, build_finding_explanation_from_report,
    build_finding_explanation_selection_from_report,
};
pub use model::{
    ExplainContext, ExplainDecision, ExplainDecisionTraceStep, ExplainReport, ExplainRoleEvidence,
    ExplainScope, ExplainSource,
};
pub use render::render_explain_report;
