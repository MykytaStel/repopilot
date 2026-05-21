pub mod fixtures;

use serde::Serialize;

#[derive(Debug, Default, Clone, Serialize, PartialEq, Eq)]
pub struct RuleEvaluationReport {
    pub rules_evaluated: usize,
    pub fixtures_total: usize,
    pub expected_findings: usize,
    pub actual_findings: usize,
    pub missing_findings: usize,
    pub unexpected_findings: usize,
    pub contract_violations: usize,
    pub stable_id_failures: usize,
    pub rules: Vec<RuleEvaluationRuleReport>,
}

#[derive(Debug, Default, Clone, Serialize, PartialEq, Eq)]
pub struct RuleEvaluationRuleReport {
    pub rule_id: String,
    pub fixtures_total: usize,
    pub expected_findings: usize,
    pub actual_findings: usize,
    pub missing_findings: usize,
    pub unexpected_findings: usize,
    pub contract_violations: usize,
    pub stable_id_failures: usize,
}

impl RuleEvaluationReport {
    pub(crate) fn add_rule(&mut self, rule: RuleEvaluationRuleReport) {
        self.fixtures_total += rule.fixtures_total;
        self.expected_findings += rule.expected_findings;
        self.actual_findings += rule.actual_findings;
        self.missing_findings += rule.missing_findings;
        self.unexpected_findings += rule.unexpected_findings;
        self.contract_violations += rule.contract_violations;
        self.stable_id_failures += rule.stable_id_failures;
        self.rules.push(rule);
        self.rules_evaluated = self.rules.len();
    }
}
