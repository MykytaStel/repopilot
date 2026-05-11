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

pub mod audits;
pub mod baseline;
pub mod compare;
pub mod config;
pub mod doctor;
pub mod findings;
pub mod frameworks;
pub mod graph;
pub mod output;
pub mod receipt;
pub mod report;
pub mod review;
pub mod rules;
pub mod scan;
