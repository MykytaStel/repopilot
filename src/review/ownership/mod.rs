mod codeowners;
mod model;

pub use codeowners::{OwnershipDiscovery, OwnershipIndex};
pub use model::{Owner, OwnershipDiagnostic, OwnershipSummary, PathOwnership};
