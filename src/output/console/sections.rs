use crate::findings::types::Finding;
use crate::frameworks::{DetectedFramework, FrameworkProject};
use crate::output::color;
use crate::output::finding_helpers::{clusters_by_rule_scope, example_locations};
use crate::output::report_stats::{ReportStats, TOOL_VERSION};
use crate::output::report_text::{console_severity_counts_text, named_counts_text};
use crate::risk::RiskPriority;
use crate::scan::types::{DiagnosticSeverity, ScanSummary};
use std::fmt::Write;

include!("sections/header.rs");
include!("sections/risk.rs");
include!("sections/inventory.rs");
