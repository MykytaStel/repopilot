pub mod ai_context;
pub mod baseline;
pub mod cache;
mod dispatch;
pub(crate) mod filters;
pub(crate) mod focus;
pub mod init;
mod llm;
pub mod mcp;
pub(crate) mod product_scan;
mod progress;
pub mod review;
pub mod scan;
pub(crate) mod scan_config;
pub mod snapshot;

pub use dispatch::{CliExit, EXIT_FINDINGS, EXIT_RUNTIME, EXIT_USAGE, run};
