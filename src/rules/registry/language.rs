use crate::findings::types::{FindingCategory, Severity};
use crate::rules::metadata::RuleMetadata;

pub(super) static RULES: &[RuleMetadata] = &[RuleMetadata {
    rule_id: "language.rust.panic-risk",
    title: "Risky Rust panic or unwrap usage",
    category: FindingCategory::CodeQuality,
    default_severity: Severity::Medium,
    docs_url: None,
    description: "Rust panic-style operations such as unwrap(), expect(), panic!, todo!, and unimplemented! can be risky in reusable production code. Their severity depends on whether the code is test code, CLI boundary code, library code, or domain code.",
    recommendation: Some(
        "Use context-aware error handling. Prefer Result, ?, typed errors, validation, or explicit fallback behavior in production and library code.",
    ),
}];
