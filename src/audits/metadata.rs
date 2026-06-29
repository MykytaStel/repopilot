use crate::findings::provenance::AnalysisScope;
use crate::findings::types::FindingCategory;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AuditKind {
    File,
    Project,
    Framework,
}

impl AuditKind {
    pub fn label(self) -> &'static str {
        match self {
            Self::File => "file",
            Self::Project => "project",
            Self::Framework => "framework",
        }
    }

    /// Evidence source and execution scope are separate contracts.
    pub fn analysis_scope(self) -> AnalysisScope {
        match self {
            Self::File => AnalysisScope::File,
            Self::Project => AnalysisScope::Repository,
            Self::Framework => AnalysisScope::FrameworkProject,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AuditMetadata {
    pub audit_id: &'static str,
    pub kind: AuditKind,
    pub category: FindingCategory,
    pub rule_ids: &'static [&'static str],
}

impl AuditMetadata {
    pub fn new(
        audit_id: &'static str,
        kind: AuditKind,
        category: FindingCategory,
        rule_ids: &'static [&'static str],
    ) -> Self {
        Self {
            audit_id,
            kind,
            category,
            rule_ids,
        }
    }

    pub fn covers_rule_id(&self, rule_id: &str) -> bool {
        self.rule_ids.contains(&rule_id)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn audit_kind_labels_are_stable() {
        assert_eq!(AuditKind::File.label(), "file");
        assert_eq!(AuditKind::Project.label(), "project");
        assert_eq!(AuditKind::Framework.label(), "framework");
        assert_eq!(AuditKind::File.analysis_scope(), AnalysisScope::File);
        assert_eq!(
            AuditKind::Project.analysis_scope(),
            AnalysisScope::Repository
        );
        assert_eq!(
            AuditKind::Framework.analysis_scope(),
            AnalysisScope::FrameworkProject
        );
    }

    #[test]
    fn audit_metadata_can_match_rule_ids() {
        let metadata = AuditMetadata::new(
            "audit.file.example",
            AuditKind::File,
            FindingCategory::CodeQuality,
            &["example.rule"],
        );

        assert!(metadata.covers_rule_id("example.rule"));
        assert!(!metadata.covers_rule_id("other.rule"));
    }
}
