use crate::knowledge::bundled_knowledge;
use crate::knowledge::model::ParadigmProfile;

pub fn profile_by_id(id: &str) -> Option<&'static ParadigmProfile> {
    bundled_knowledge()
        .paradigms
        .iter()
        .find(|paradigm| paradigm.id == id)
}
