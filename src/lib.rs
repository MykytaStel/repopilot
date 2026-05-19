//! RepoPilot is a local-first repository audit CLI.
//!
//! It scans repositories, detects architecture, testing, code-quality, and
//! security risks, supports project-level configuration, and can compare
//! current findings against a baseline for CI-friendly workflows.
//!
//! Most users interact with RepoPilot through the CLI:
//!
//! ```text
//! repopilot init
//! repopilot scan .
//! repopilot review .
//! repopilot baseline create .
//! ```

pub mod api;

#[doc(hidden)]
pub mod audits;
#[doc(hidden)]
pub mod baseline;
#[doc(hidden)]
pub mod compare;
pub mod config;
#[doc(hidden)]
pub mod doctor;
#[doc(hidden)]
pub mod explain;
pub mod findings;
#[doc(hidden)]
pub mod frameworks;
#[doc(hidden)]
pub mod graph;
#[doc(hidden)]
pub mod knowledge;
pub mod output;
#[doc(hidden)]
pub mod receipt;
pub mod report;
pub mod review;
pub mod risk;
#[doc(hidden)]
pub mod rules;
pub mod scan;
