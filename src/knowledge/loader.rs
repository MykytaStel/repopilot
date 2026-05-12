use crate::knowledge::model::{KnowledgeBase, KnowledgePack};
use crate::knowledge::validate::{KnowledgeValidationError, validate_knowledge_base};
use std::sync::OnceLock;

const CORE_PACK: &str = include_str!("packs/core.toml");

static KNOWLEDGE: OnceLock<KnowledgeBase> = OnceLock::new();

pub fn bundled_knowledge() -> &'static KnowledgeBase {
    KNOWLEDGE.get_or_init(|| {
        load_from_str(CORE_PACK)
            .unwrap_or_else(|error| panic!("bundled knowledge pack is invalid: {error}"))
    })
}

pub fn load_from_str(content: &str) -> Result<KnowledgeBase, KnowledgeLoadError> {
    let pack: KnowledgePack = toml::from_str(content).map_err(KnowledgeLoadError::Parse)?;
    let base = KnowledgeBase::from(pack);
    validate_knowledge_base(&base).map_err(KnowledgeLoadError::Invalid)?;
    Ok(base)
}

#[derive(Debug)]
pub enum KnowledgeLoadError {
    Parse(toml::de::Error),
    Invalid(KnowledgeValidationError),
}

impl std::fmt::Display for KnowledgeLoadError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            KnowledgeLoadError::Parse(error) => write!(formatter, "failed to parse TOML: {error}"),
            KnowledgeLoadError::Invalid(error) => write!(formatter, "{error}"),
        }
    }
}

impl std::error::Error for KnowledgeLoadError {}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn bundled_knowledge_pack_loads() {
        let knowledge = bundled_knowledge();

        assert!(
            knowledge
                .languages
                .iter()
                .any(|language| language.id == "rust")
        );
        assert!(
            knowledge
                .rule_applicability
                .iter()
                .any(|rule| rule.rule_id == "language.rust.panic-risk")
        );
    }

    #[test]
    fn invalid_pack_fixture_fails_validation() {
        let invalid = r#"
            [[languages]]
            id = "rust"
            name = "Rust"
            extensions = ["rs"]
            support = "rule-aware"

            [[languages]]
            id = "rust"
            name = "Rust duplicate"
            extensions = ["rs2"]
            support = "detect-only"
        "#;

        assert!(load_from_str(invalid).is_err());
    }
}
