use crate::knowledge::bundled_knowledge;
use crate::knowledge::model::RuntimeProfile;

pub fn profile_by_id(id: &str) -> Option<&'static RuntimeProfile> {
    bundled_knowledge()
        .runtimes
        .iter()
        .find(|runtime| runtime.id == id)
}
