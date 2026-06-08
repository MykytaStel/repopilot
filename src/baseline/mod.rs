pub mod diff;
pub mod gate;
pub mod key;
pub mod model;
pub mod reader;
pub mod writer;

pub use diff::{
    BaselineScanReport, BaselineStatus, all_findings_new, diff_summary_against_baseline,
};
pub use key::{normalized_relative_path, stable_finding_key};
pub use model::Baseline;
