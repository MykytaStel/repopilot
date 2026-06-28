use super::context::AuditContext;
use super::kinds::FileRole;

/// Repository evidence used to assign a semantic file role.
///
/// This is separate from finding-detector provenance. A finding may be
/// AST-backed while its surrounding file role is justified by a path,
/// framework marker, manifest declaration, content marker, or a combination.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RoleEvidenceSource {
    Path,
    Content,
    Framework,
    Manifest,
    Signal,
    Mixed,
    Fallback,
}

impl RoleEvidenceSource {
    pub fn as_id(self) -> &'static str {
        match self {
            RoleEvidenceSource::Path => "path",
            RoleEvidenceSource::Content => "content",
            RoleEvidenceSource::Framework => "framework",
            RoleEvidenceSource::Manifest => "manifest",
            RoleEvidenceSource::Signal => "signal",
            RoleEvidenceSource::Mixed => "mixed",
            RoleEvidenceSource::Fallback => "fallback",
        }
    }
}

/// Explainable evidence for one assigned [`FileRole`].
///
/// Reasons are stable product-facing descriptions, not arbitrary source
/// snippets. Decision-trace surfaces can render them without copying the role
/// classifier's heuristics.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RoleEvidence {
    pub role: FileRole,
    pub source: RoleEvidenceSource,
    pub reason: &'static str,
}

impl RoleEvidence {
    pub const fn new(role: FileRole, source: RoleEvidenceSource, reason: &'static str) -> Self {
        Self {
            role,
            source,
            reason,
        }
    }
}

/// The audit context plus the evidence that justified every file role.
///
/// `classify_file` remains the compatibility API for audits that only need the
/// semantic context. Scanner/cache/explain surfaces use the detailed API when
/// role provenance must survive beyond classification.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FileContextClassification {
    pub context: AuditContext,
    pub role_evidence: Vec<RoleEvidence>,
}

impl FileContextClassification {
    pub fn evidence_for_role(&self, role: FileRole) -> Option<&RoleEvidence> {
        self.role_evidence
            .iter()
            .find(|evidence| evidence.role == role)
    }
}
