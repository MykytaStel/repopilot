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
//!
//! Rust embedding consumers should use the `api` facade. Other modules are
//! implementation detail for the CLI and may change during the pre-1.0 line.

pub mod analysis;
pub mod api;
pub mod error;

#[doc(hidden)]
pub mod audits;
#[doc(hidden)]
pub mod baseline;
#[doc(hidden)]
pub mod config;
#[doc(hidden)]
pub mod explain;
#[doc(hidden)]
pub mod findings;
#[doc(hidden)]
pub mod frameworks;
#[doc(hidden)]
pub mod graph;
#[doc(hidden)]
pub mod knowledge;
#[doc(hidden)]
pub mod output;
#[doc(hidden)]
pub mod receipt;
#[doc(hidden)]
pub mod report;
#[doc(hidden)]
pub mod review;
#[doc(hidden)]
pub mod risk;
#[doc(hidden)]
pub mod rules;
#[doc(hidden)]
pub mod scan;
