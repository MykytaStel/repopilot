//! Runtime-risk tables for Java: the emitters live with the pattern
//! definitions in `audits::code_quality::language_risk`.

use crate::audits::code_quality::language_risk::pattern::managed;
use crate::audits::code_quality::language_risk::tables::{RiskLineSanitizer, RiskTables};

pub(super) static JAVA_RISK: RiskTables = RiskTables {
    emit_node: managed::emit_java_node,
    emit_line: managed::emit_line_jvm,
    sanitizer: RiskLineSanitizer::CStyle,
};
