use crate::knowledge::bundled_knowledge;
use crate::knowledge::model::RuleApplicability;

pub fn applicability_for_rule(rule_id: &str) -> Option<&'static RuleApplicability> {
    bundled_knowledge()
        .rule_applicability
        .iter()
        .find(|rule| rule.rule_id == rule_id)
}
