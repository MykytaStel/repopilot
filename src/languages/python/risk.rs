//! Runtime-risk tables for Python: the emitters live with the pattern
//! definitions in `audits::code_quality::language_risk`.

use crate::audits::code_quality::language_risk::pattern::python;
use crate::audits::code_quality::language_risk::tables::{RiskLineSanitizer, RiskTables};

pub(super) static PYTHON_RISK: RiskTables = RiskTables {
    emit_node: python::emit_python_node,
    emit_line: python::emit_line,
    sanitizer: RiskLineSanitizer::Python,
};
