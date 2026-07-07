use crate::analysis::parse::ParsedFile;
use crate::audits::metadata::{AuditKind, AuditMetadata};
use crate::audits::traits::{FileAudit, ProjectAudit};
use crate::findings::provenance::AnalysisScope;
use crate::findings::types::{Finding, FindingCategory};
use crate::rules::{RuleRequirements, lookup_rule_metadata};
use crate::scan::config::ScanConfig;
use crate::scan::facts::{FileFacts, ScanFacts};

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

    pub(crate) fn run(&self, file: &FileFacts, config: &ScanConfig) -> Vec<Finding> {
        stamp_findings_analysis_scope(self.audit.audit(file, config), self.scope())
    }

    pub(crate) fn run_parsed(
        &self,
        file: &FileFacts,
        parsed: &ParsedFile,
        config: &ScanConfig,
    ) -> Vec<Finding> {
        stamp_findings_analysis_scope(self.audit.audit_parsed(file, parsed, config), self.scope())
    }

    pub(crate) fn requires_parsed_syntax(&self) -> bool {
        self.requirements().requires_parsed_syntax()
    }

    pub(crate) fn requirements(&self) -> RuleRequirements {
        let mut requirements = None;

        for rule_id in self.metadata.rule_ids {
            let Some(rule) = lookup_rule_metadata(rule_id) else {
                continue;
            };
            let rule_requirements = rule.requirements;
            if let Some(previous) = requirements {
                debug_assert_eq!(
                    previous, rule_requirements,
                    "audit {} mixes incompatible rule requirements across rule ids",
                    self.metadata.audit_id
                );
            } else {
                requirements = Some(rule_requirements);
            }
        }

        requirements.unwrap_or(RuleRequirements::UNDECLARED)
    }

    fn scope(&self) -> AnalysisScope {
        self.metadata.kind.analysis_scope()
    }

    pub(super) fn into_audit(self) -> Box<dyn FileAudit> {
        Box::new(ScopedFileAudit {
            scope: self.scope(),
            audit: self.audit,
        })
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

    pub(super) fn run(&self, facts: &ScanFacts, config: &ScanConfig) -> Vec<Finding> {
        stamp_findings_analysis_scope(
            self.audit.audit(facts, config),
            self.metadata.kind.analysis_scope(),
        )
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

    pub(super) fn run(&self, facts: &ScanFacts, config: &ScanConfig) -> Vec<Finding> {
        stamp_findings_analysis_scope(
            self.audit.audit(facts, config),
            self.metadata.kind.analysis_scope(),
        )
    }
}

struct ScopedFileAudit {
    scope: AnalysisScope,
    audit: Box<dyn FileAudit>,
}

impl FileAudit for ScopedFileAudit {
    fn audit(&self, file: &FileFacts, config: &ScanConfig) -> Vec<Finding> {
        stamp_findings_analysis_scope(self.audit.audit(file, config), self.scope)
    }

    fn audit_parsed(
        &self,
        file: &FileFacts,
        parsed: &ParsedFile,
        config: &ScanConfig,
    ) -> Vec<Finding> {
        stamp_findings_analysis_scope(self.audit.audit_parsed(file, parsed, config), self.scope)
    }
}

pub(crate) fn stamp_findings_analysis_scope(
    mut findings: Vec<Finding>,
    scope: AnalysisScope,
) -> Vec<Finding> {
    for finding in &mut findings {
        finding.provenance.analysis_scope = scope;
    }
    findings
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
