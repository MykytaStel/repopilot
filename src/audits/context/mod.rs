pub mod classify;
pub mod model;

pub use classify::{classify_file, classify_file_with_evidence};
pub use model::{
    AuditContext, FileContextClassification, FileRole, FrameworkKind, LanguageKind,
    ProgrammingParadigm, RoleEvidence, RoleEvidenceSource, RuntimeKind,
};
