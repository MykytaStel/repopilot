use crate::knowledge::active_knowledge;
use crate::knowledge::model::RuntimeProfile;

pub fn profile_by_id(id: &str) -> Option<&'static RuntimeProfile> {
    active_knowledge()
        .runtimes
        .iter()
        .find(|runtime| runtime.id == id)
}
