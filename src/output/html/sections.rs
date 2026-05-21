use super::escape::escape_html;
use crate::baseline::diff::BaselineScanReport;
use crate::baseline::gate::CiGateResult;
use crate::findings::types::Severity;
use crate::output::report_stats::{ReportStats, severity_order};
use crate::scan::types::ScanSummary;

mod frameworks;

include!("sections/meta.rs");
include!("sections/cards.rs");
include!("sections/risk.rs");
include!("sections/inventory.rs");
