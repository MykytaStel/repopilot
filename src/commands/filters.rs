use crate::cli::{ConfidenceArg, PriorityArg, ReviewOptions, ScanOptions, SeverityArg};
use repopilot::findings::filter::FindingFilter;
use repopilot::findings::types::{Confidence, Severity};
use repopilot::risk::RiskPriority;

pub fn severity_arg_into(arg: SeverityArg) -> Severity {
    match arg {
        SeverityArg::Info => Severity::Info,
        SeverityArg::Low => Severity::Low,
        SeverityArg::Medium => Severity::Medium,
        SeverityArg::High => Severity::High,
        SeverityArg::Critical => Severity::Critical,
    }
}

pub fn confidence_arg_into(arg: ConfidenceArg) -> Confidence {
    arg.into()
}

pub fn priority_arg_into(arg: PriorityArg) -> RiskPriority {
    arg.into()
}

pub fn scan_pre_visibility_filter(options: &ScanOptions) -> FindingFilter {
    FindingFilter {
        min_severity: options.min_severity.map(severity_arg_into),
        min_confidence: options.min_confidence.map(confidence_arg_into),
        min_priority: None,
        rule_ids: options.rule.clone(),
    }
}

pub fn scan_priority_filter(options: &ScanOptions) -> FindingFilter {
    FindingFilter {
        min_priority: options.min_priority.map(priority_arg_into),
        ..FindingFilter::default()
    }
}

pub fn review_pre_diff_filter(options: &ReviewOptions) -> FindingFilter {
    FindingFilter {
        min_severity: options.min_severity.map(severity_arg_into),
        min_confidence: options.min_confidence.map(confidence_arg_into),
        min_priority: None,
        rule_ids: Vec::new(),
    }
}

pub fn review_priority_filter(options: &ReviewOptions) -> FindingFilter {
    FindingFilter {
        min_priority: options.min_priority.map(priority_arg_into),
        ..FindingFilter::default()
    }
}
