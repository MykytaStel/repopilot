use crate::knowledge::active_knowledge;
use crate::knowledge::model::ParadigmProfile;

pub fn profile_by_id(id: &str) -> Option<&'static ParadigmProfile> {
    active_knowledge()
        .paradigms
        .iter()
        .find(|paradigm| paradigm.id == id)
}
