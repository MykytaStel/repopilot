mod codeowners;
mod model;

pub use codeowners::{OwnershipDiscovery, OwnershipIndex};
pub use model::{Owner, OwnershipAssessment, OwnershipDiagnostic, OwnershipSummary, PathOwnership};
