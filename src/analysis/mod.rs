mod model;
#[doc(hidden)]
pub mod parse;
pub mod path_classifier;

pub use model::{ArchitectureContext, FileRole, LanguageFamily, ModuleKind};
pub use path_classifier::{ArchitectureClassifier, classify_file_architecture};
