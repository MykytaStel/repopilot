mod file_audits;
mod framework_audits;
mod project_audits;
mod registration;

pub use file_audits::{build_file_audits, registered_file_audits};
pub use framework_audits::{registered_framework_audits, run_framework_audits};
pub use project_audits::{registered_project_audits, run_project_audits};
pub use registration::{
    FileAuditRegistration, FrameworkAuditRegistration, ProjectAuditRegistration,
};

#[cfg(test)]
mod tests {
    use super::*;
    use crate::audits::metadata::AuditKind;
    use crate::frameworks::DetectedFramework;
    use crate::rules::lookup_rule_metadata;
    use crate::scan::config::ScanConfig;
    use crate::scan::facts::ScanFacts;
    use std::collections::HashSet;

    #[test]
    fn registered_file_audits_have_rule_metadata() {
        let config = ScanConfig {
            detect_secret_like_names: true,
            ..ScanConfig::default()
        };

        assert_registrations_have_rule_metadata(
            registered_file_audits(&config)
                .iter()
                .map(|registration| registration.metadata.clone()),
            AuditKind::File,
        );
    }

    #[test]
    fn registered_project_audits_have_rule_metadata() {
        let config = ScanConfig {
            detect_missing_tests: true,
            ..ScanConfig::default()
        };

        assert_registrations_have_rule_metadata(
            registered_project_audits(&config)
                .iter()
                .map(|registration| registration.metadata.clone()),
            AuditKind::Project,
        );
    }

    #[test]
    fn registered_framework_audits_have_rule_metadata() {
        let facts = ScanFacts {
            detected_frameworks: vec![
                DetectedFramework::ReactNative { version: None },
                DetectedFramework::React { version: None },
            ],
            ..ScanFacts::default()
        };

        assert_registrations_have_rule_metadata(
            registered_framework_audits(&facts)
                .iter()
                .map(|registration| registration.metadata.clone()),
            AuditKind::Framework,
        );
    }

    #[test]
    fn registered_audit_ids_are_unique_within_each_scope() {
        let config = ScanConfig {
            detect_missing_tests: true,
            detect_secret_like_names: true,
            ..ScanConfig::default()
        };
        let framework_facts = ScanFacts {
            detected_frameworks: vec![
                DetectedFramework::ReactNative { version: None },
                DetectedFramework::React { version: None },
            ],
            ..ScanFacts::default()
        };

        assert_unique_audit_ids(
            registered_file_audits(&config)
                .iter()
                .map(|registration| registration.metadata.clone()),
        );
        assert_unique_audit_ids(
            registered_project_audits(&config)
                .iter()
                .map(|registration| registration.metadata.clone()),
        );
        assert_unique_audit_ids(
            registered_framework_audits(&framework_facts)
                .iter()
                .map(|registration| registration.metadata.clone()),
        );
    }

    fn assert_registrations_have_rule_metadata(
        registrations: impl Iterator<Item = crate::audits::metadata::AuditMetadata>,
        expected_kind: AuditKind,
    ) {
        for metadata in registrations {
            assert_eq!(
                metadata.kind, expected_kind,
                "audit {} has unexpected kind",
                metadata.audit_id
            );
            assert!(
                !metadata.rule_ids.is_empty(),
                "audit {} should declare at least one rule_id",
                metadata.audit_id
            );

            for rule_id in metadata.rule_ids {
                let rule = lookup_rule_metadata(rule_id).unwrap_or_else(|| {
                    panic!(
                        "audit {} references missing rule metadata: {}",
                        metadata.audit_id, rule_id
                    )
                });
                assert_eq!(
                    rule.category, metadata.category,
                    "audit {} category does not match rule {}",
                    metadata.audit_id, rule_id
                );
            }
        }
    }

    fn assert_unique_audit_ids(
        registrations: impl Iterator<Item = crate::audits::metadata::AuditMetadata>,
    ) {
        let mut seen = HashSet::new();

        for metadata in registrations {
            assert!(
                seen.insert(metadata.audit_id),
                "duplicate audit_id: {}",
                metadata.audit_id
            );
        }
    }
}
