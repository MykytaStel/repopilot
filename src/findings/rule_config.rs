//! Applies the user's `[rules]` configuration to enriched findings.
//!
//! Runs after enrichment (so the registry contract has already been applied)
//! and before risk scoring (so priorities reflect the overridden severity).
//! A severity override wins over the registry default, audit tiers, and
//! generic knowledge-pack adjustments — but a path-scoped local overlay
//! decision (`.repopilot/overlay.toml`) is more specific and wins over this
//! global override instead of being clobbered by it.

use crate::findings::types::Finding;
use crate::scan::config::ScanConfig;
use crate::scan::types::ScanDiagnostic;

pub fn apply_rule_config(findings: &mut Vec<Finding>, config: &ScanConfig) {
    if config.disabled_rules.is_empty() && config.severity_overrides.is_empty() {
        return;
    }

    findings.retain(|finding| !config.disabled_rules.contains(&finding.rule_id));

    for finding in findings.iter_mut() {
        let overlay_applied = finding
            .provenance
            .knowledge_decision
            .as_ref()
            .is_some_and(|decision| decision.overlay_applied);
        if overlay_applied {
            continue;
        }
        if let Some(severity) = config.severity_overrides.get(&finding.rule_id) {
            finding.severity = *severity;
        }
    }
}

pub fn rule_config_diagnostics(config: &ScanConfig) -> Vec<ScanDiagnostic> {
    config
        .rule_config_problems
        .iter()
        .map(|problem| ScanDiagnostic::warning("config.rules", problem.clone()))
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::findings::types::{Finding, Severity};

    fn finding(rule_id: &str, severity: Severity) -> Finding {
        Finding {
            rule_id: rule_id.to_string(),
            severity,
            ..Finding::default()
        }
    }

    #[test]
    fn disabled_rules_are_dropped_and_overrides_rewrite_severity() {
        let mut config = ScanConfig::default();
        config.disabled_rules.insert("code-marker.todo".to_string());
        config
            .severity_overrides
            .insert("architecture.large-file".to_string(), Severity::Low);

        let mut findings = vec![
            finding("code-marker.todo", Severity::Low),
            finding("architecture.large-file", Severity::High),
            finding("architecture.circular-dependency", Severity::High),
        ];

        apply_rule_config(&mut findings, &config);

        assert_eq!(findings.len(), 2);
        assert_eq!(findings[0].rule_id, "architecture.large-file");
        assert_eq!(findings[0].severity, Severity::Low);
        assert_eq!(findings[1].severity, Severity::High);
    }

    #[test]
    fn empty_rule_config_leaves_findings_untouched() {
        let config = ScanConfig::default();
        let mut findings = vec![finding("code-marker.todo", Severity::Low)];
        apply_rule_config(&mut findings, &config);
        assert_eq!(findings.len(), 1);
    }

    #[test]
    fn problems_become_warning_diagnostics() {
        let mut config = ScanConfig::default();
        config
            .rule_config_problems
            .push("[rules] disable lists unknown rule id `nope`; entry ignored".to_string());

        let diagnostics = rule_config_diagnostics(&config);

        assert_eq!(diagnostics.len(), 1);
        assert_eq!(diagnostics[0].code, "config.rules");
        assert!(diagnostics[0].message.contains("`nope`"));
    }

    #[test]
    fn repopilot_toml_severity_override_is_skipped_when_overlay_already_decided() {
        use crate::findings::provenance::{KnowledgeDecisionAction, KnowledgeDecisionProvenance};

        let mut config = ScanConfig::default();
        config
            .severity_overrides
            .insert("architecture.large-file".to_string(), Severity::High);

        let mut overlay_decided = finding("architecture.large-file", Severity::Low);
        overlay_decided.provenance.knowledge_decision = Some(KnowledgeDecisionProvenance {
            base_severity: Severity::High,
            signal: None,
            action: KnowledgeDecisionAction::Downgrade,
            decided_severity: Severity::Low,
            reason: Some("overlay[1] matched".to_string()),
            overlay_applied: true,
        });

        let mut findings = vec![overlay_decided];
        apply_rule_config(&mut findings, &config);

        assert_eq!(
            findings[0].severity,
            Severity::Low,
            "repopilot.toml must not clobber a severity overlay already decided"
        );
    }
}
