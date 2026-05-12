use crate::knowledge::bundled_knowledge;
use crate::knowledge::model::FrameworkProfile;

pub fn profile_by_id(id: &str) -> Option<&'static FrameworkProfile> {
    bundled_knowledge()
        .frameworks
        .iter()
        .find(|framework| framework.id == id)
}
