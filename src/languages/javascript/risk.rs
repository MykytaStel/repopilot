//! Runtime-risk tables for the JS dialect family: the emitters live with the pattern
//! definitions in `audits::code_quality::language_risk`.

use crate::audits::code_quality::language_risk::pattern::js;
use crate::audits::code_quality::language_risk::tables::{RiskLineSanitizer, RiskTables};

pub(super) static JS_FAMILY_RISK: RiskTables = RiskTables {
    emit_node: js::emit_js_node,
    emit_line: js::emit_line,
    sanitizer: RiskLineSanitizer::CStyle,
};
