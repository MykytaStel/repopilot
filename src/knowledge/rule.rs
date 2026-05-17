use crate::knowledge::active_knowledge;
use crate::knowledge::model::RuleApplicability;

pub fn applicability_for_rule(rule_id: &str) -> Option<&'static RuleApplicability> {
    active_knowledge()
        .rule_applicability
        .iter()
        .find(|rule| rule.rule_id == rule_id)
}
