//! Stable crate facade for consumers embedding RepoPilot.
//!
//! The CLI uses more internal modules while the library API intentionally keeps a
//! small surface around scan, review, configuration, reporting, and risk models.

pub mod config {
    pub use crate::config::loader::{load_default_config, load_optional_config};
    pub use crate::config::model::RepoPilotConfig;
    pub use crate::config::presets::{Preset, apply_preset};
}

pub mod findings {
    pub use crate::findings::filter::{FindingFilter, recompute_summary_metrics};
    pub use crate::findings::types::{Confidence, Evidence, Finding, FindingCategory, Severity};
    pub use crate::findings::visibility::{FindingVisibilityProfile, apply_visibility_profile};
}

pub mod report {
    pub use crate::output::{OutputFormat, render_baseline_scan_report, render_scan_summary};
    pub use crate::receipt::{build_audit_receipt, render_receipt_json};
    pub use crate::report::schema::{
        BaselineJsonReport, REPOPILOT_VERSION, ReportEnvelope, ReviewJsonReport,
        SCAN_REPORT_SCHEMA_VERSION, ScanJsonReport, parse_scan_summary_json,
        parse_scan_summary_value,
    };
    pub use crate::report::writer::write_report;
}

pub mod review {
    pub use crate::review::diff::{ChangeStatus, ChangedFile};
    pub use crate::review::model::{ReviewFindingStatus, ReviewReport};
    pub use crate::review::{build_review_report, review_report_for_ci};
}

pub mod risk {
    pub use crate::risk::{
        FORMULA_VERSION, RiskAssessment, RiskFormula, RiskInputs, RiskPriority, RiskSignal,
        RiskSummary, assess_finding, assess_findings, priority_for_score,
    };
}

pub mod scan {
    pub use crate::scan::config::ScanConfig;
    pub use crate::scan::scanner::{
        ScanEngine, collect_scan_facts, collect_scan_facts_with_config, scan_changed_with_config,
        scan_path, scan_path_with_config,
    };
    pub use crate::scan::types::{
        DiagnosticSeverity, LanguageSummary, ScanDiagnostic, ScanMode, ScanSummary, ScanTimings,
    };
    pub use crate::scan::workspace_scan::scan_workspace_with_config;
}
