pub mod builder;
pub mod model;
pub mod render;

pub use builder::{build_explain_report, build_explain_report_with_root};
pub use model::{
    ExplainContext, ExplainDecision, ExplainDecisionTraceStep, ExplainReport, ExplainRoleEvidence,
    ExplainScope, ExplainSource,
};
pub use render::render_explain_report;
