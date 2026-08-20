pub(crate) mod api_contract;
pub(crate) mod artifact;
pub(crate) mod exports;
mod model;
#[doc(hidden)]
pub mod parse;
pub mod path_classifier;
pub(crate) mod symbols;

pub(crate) use artifact::{FileContextFacts, ParsedArtifact, RoleEvidenceFact, SyntaxSummary};
pub use model::{ArchitectureContext, FileRole, LanguageFamily, ModuleKind};
pub use path_classifier::{ArchitectureClassifier, classify_file_architecture};
