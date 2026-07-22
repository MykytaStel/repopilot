//! Runtime-risk tables for Go: the emitters live with the pattern
//! definitions in `audits::code_quality::language_risk`.

use crate::audits::code_quality::language_risk::pattern::go;
use crate::audits::code_quality::language_risk::tables::{RiskLineSanitizer, RiskTables};

pub(super) static GO_RISK: RiskTables = RiskTables {
    emit_node: go::emit_go_node,
    emit_line: go::emit_line,
    sanitizer: RiskLineSanitizer::CStyle,
};
