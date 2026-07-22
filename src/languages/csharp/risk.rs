//! Runtime-risk tables for C#: the emitters live with the pattern
//! definitions in `audits::code_quality::language_risk`.

use crate::audits::code_quality::language_risk::pattern::managed;
use crate::audits::code_quality::language_risk::tables::{RiskLineSanitizer, RiskTables};

pub(super) static CSHARP_RISK: RiskTables = RiskTables {
    emit_node: managed::emit_csharp_node,
    emit_line: managed::emit_line_csharp,
    sanitizer: RiskLineSanitizer::CStyle,
};
