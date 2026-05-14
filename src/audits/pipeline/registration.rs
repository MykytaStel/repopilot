use crate::audits::metadata::{AuditKind, AuditMetadata};
use crate::audits::traits::{FileAudit, ProjectAudit};
use crate::findings::types::FindingCategory;

pub const CODE_MARKER_RULES: &[&str] =
    &["code-marker.todo", "code-marker.fixme", "code-marker.hack"];
pub const LANGUAGE_RISK_RULES: &[&str] = &[
    "language.go.panic-exit-risk",
    "language.python.exception-risk",
    "language.javascript.runtime-exit-risk",
    "language.managed.fatal-exception-risk",
];
pub const RN_DEP_HEALTH_RULES: &[&str] = &[
    "framework.rn-async-storage-legacy",
    "framework.rn-navigation-compat",
    "framework.rn-reanimated-compat",
    "framework.rn-gesture-handler-old",
    "framework.rn-new-arch-incompatible-dep",
];

pub struct FileAuditRegistration {
    pub metadata: AuditMetadata,
    pub(super) audit: Box<dyn FileAudit>,
}

impl FileAuditRegistration {
    pub(super) fn new(metadata: AuditMetadata, audit: Box<dyn FileAudit>) -> Self {
        Self { metadata, audit }
    }

    pub(super) fn into_audit(self) -> Box<dyn FileAudit> {
        self.audit
    }
}

pub struct ProjectAuditRegistration {
    pub metadata: AuditMetadata,
    pub(super) audit: Box<dyn ProjectAudit>,
}

impl ProjectAuditRegistration {
    pub(super) fn new(metadata: AuditMetadata, audit: Box<dyn ProjectAudit>) -> Self {
        Self { metadata, audit }
    }
}

pub struct FrameworkAuditRegistration {
    pub metadata: AuditMetadata,
    pub(super) audit: Box<dyn ProjectAudit>,
}

impl FrameworkAuditRegistration {
    pub(super) fn new(metadata: AuditMetadata, audit: Box<dyn ProjectAudit>) -> Self {
        Self { metadata, audit }
    }
}

pub(super) fn file_metadata(
    audit_id: &'static str,
    category: FindingCategory,
    rule_ids: &'static [&'static str],
) -> AuditMetadata {
    AuditMetadata::new(audit_id, AuditKind::File, category, rule_ids)
}

pub(super) fn project_metadata(
    audit_id: &'static str,
    category: FindingCategory,
    rule_ids: &'static [&'static str],
) -> AuditMetadata {
    AuditMetadata::new(audit_id, AuditKind::Project, category, rule_ids)
}

pub(super) fn framework_metadata(
    audit_id: &'static str,
    rule_ids: &'static [&'static str],
) -> AuditMetadata {
    AuditMetadata::new(
        audit_id,
        AuditKind::Framework,
        FindingCategory::Framework,
        rule_ids,
    )
}
