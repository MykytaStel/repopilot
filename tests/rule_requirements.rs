use repopilot::audits::metadata::AuditKind;
use repopilot::audits::pipeline::{
    registered_file_audits, registered_framework_audits, registered_project_audits,
};
use repopilot::frameworks::DetectedFramework;
use repopilot::rules::{
    RuleCachePolicy, RuleOutputKind, RuleScope, SignalSource, all_rule_metadata,
    lookup_rule_metadata,
};
use repopilot::scan::config::ScanConfig;
use repopilot::scan::facts::ScanFacts;

#[test]
fn every_registered_rule_declares_a_complete_requirement_contract() {
    for rule in all_rule_metadata() {
        assert!(
            rule.requirements.is_declared(),
            "{} has undeclared requirements",
            rule.rule_id
        );
        assert_eq!(
            rule.requirements.lifecycle, rule.lifecycle,
            "{} lifecycle differs between metadata and requirements",
            rule.rule_id
        );
        assert!(
            rule.requirements
                .produces
                .contains(&RuleOutputKind::Finding),
            "{} must declare finding output",
            rule.rule_id
        );

        match rule.requirements.scope {
            RuleScope::File => assert_eq!(
                rule.requirements.cache_policy,
                RuleCachePolicy::PerFileContent,
                "{} file rule needs per-file-content cache policy",
                rule.rule_id
            ),
            RuleScope::Repository | RuleScope::FrameworkProject => assert_eq!(
                rule.requirements.cache_policy,
                RuleCachePolicy::PerWorkspaceRevision,
                "{} workspace rule needs per-workspace-revision cache policy",
                rule.rule_id
            ),
            RuleScope::ChangeSet => assert_eq!(
                rule.requirements.cache_policy,
                RuleCachePolicy::PerChangeSet,
                "{} change rule needs per-change-set cache policy",
                rule.rule_id
            ),
        }

        if rule.signal_source == SignalSource::ImportGraph {
            assert_eq!(
                rule.requirements.scope,
                RuleScope::Repository,
                "{} import-graph rule must be repository-scoped",
                rule.rule_id
            );
        }
    }
}

#[test]
fn requirements_match_registered_audit_execution_scope() {
    let config = ScanConfig {
        detect_secret_like_names: true,
        detect_missing_tests: true,
        ..ScanConfig::default()
    };

    assert_scope(
        registered_file_audits(&config)
            .iter()
            .flat_map(|registration| registration.metadata.rule_ids.iter().copied()),
        AuditKind::File,
    );
    assert_scope(
        registered_project_audits(&config)
            .iter()
            .flat_map(|registration| registration.metadata.rule_ids.iter().copied()),
        AuditKind::Project,
    );

    let rn_and_django = ScanFacts {
        detected_frameworks: vec![
            DetectedFramework::ReactNative { version: None },
            DetectedFramework::Django { version: None },
        ],
        ..ScanFacts::default()
    };
    assert_scope(
        registered_framework_audits(&rn_and_django)
            .iter()
            .flat_map(|registration| registration.metadata.rule_ids.iter().copied()),
        AuditKind::Framework,
    );

    let react_only = ScanFacts {
        detected_frameworks: vec![DetectedFramework::React { version: None }],
        ..ScanFacts::default()
    };
    assert_scope(
        registered_framework_audits(&react_only)
            .iter()
            .flat_map(|registration| registration.metadata.rule_ids.iter().copied()),
        AuditKind::Framework,
    );
}

fn assert_scope(rule_ids: impl Iterator<Item = &'static str>, kind: AuditKind) {
    let expected = match kind {
        AuditKind::File => RuleScope::File,
        AuditKind::Project => RuleScope::Repository,
        AuditKind::Framework => RuleScope::FrameworkProject,
    };

    for rule_id in rule_ids {
        let rule = lookup_rule_metadata(rule_id)
            .unwrap_or_else(|| panic!("missing rule metadata for {rule_id}"));
        assert_eq!(
            rule.requirements.scope, expected,
            "{rule_id} requirements disagree with its registered audit scope"
        );
    }
}
