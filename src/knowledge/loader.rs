use crate::knowledge::model::{KnowledgeBase, KnowledgePack};
use crate::knowledge::validate::{KnowledgeValidationError, validate_knowledge_base};
use std::sync::OnceLock;

const CORE_PACK: &str = include_str!("packs/core.toml");
const CORE_PACK_SOURCE: KnowledgePackSource<'static> =
    KnowledgePackSource::new("bundled:core", CORE_PACK);

static KNOWLEDGE: OnceLock<KnowledgeBase> = OnceLock::new();

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct KnowledgePackSource<'a> {
    pub name: &'a str,
    pub content: &'a str,
}

impl<'a> KnowledgePackSource<'a> {
    pub const fn new(name: &'a str, content: &'a str) -> Self {
        Self { name, content }
    }
}

/// Returns the runtime Knowledge Engine source for this release line.
///
/// RepoPilot 0.12 keeps the bundled pack as the only runtime source. The
/// indirection keeps rule decisions independent from how future local overlays
/// are loaded and validated.
pub fn active_knowledge() -> &'static KnowledgeBase {
    bundled_knowledge()
}

pub fn bundled_knowledge() -> &'static KnowledgeBase {
    KNOWLEDGE.get_or_init(|| {
        load_from_source(CORE_PACK_SOURCE)
            .unwrap_or_else(|error| panic!("bundled knowledge pack is invalid: {error}"))
    })
}

pub fn load_from_str(content: &str) -> Result<KnowledgeBase, KnowledgeLoadError> {
    load_from_source(KnowledgePackSource::new("inline", content))
}

pub fn load_from_source(
    source: KnowledgePackSource<'_>,
) -> Result<KnowledgeBase, KnowledgeLoadError> {
    load_from_sources(&[source])
}

pub fn load_from_sources(
    sources: &[KnowledgePackSource<'_>],
) -> Result<KnowledgeBase, KnowledgeLoadError> {
    let mut merged = KnowledgePack::default();

    for source in sources {
        let pack: KnowledgePack =
            toml::from_str(source.content).map_err(|error| KnowledgeLoadError::Parse {
                source: source.name.to_string(),
                error,
            })?;
        merge_pack(&mut merged, pack);
    }

    let base = KnowledgeBase::from(merged);
    validate_knowledge_base(&base).map_err(KnowledgeLoadError::Invalid)?;
    Ok(base)
}

fn merge_pack(target: &mut KnowledgePack, source: KnowledgePack) {
    target.languages.extend(source.languages);
    target.frameworks.extend(source.frameworks);
    target.runtimes.extend(source.runtimes);
    target.paradigms.extend(source.paradigms);
    target.rule_applicability.extend(source.rule_applicability);
}

#[derive(Debug)]
pub enum KnowledgeLoadError {
    Parse {
        source: String,
        error: toml::de::Error,
    },
    Invalid(KnowledgeValidationError),
}

impl std::fmt::Display for KnowledgeLoadError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            KnowledgeLoadError::Parse { source, error } => {
                write!(formatter, "failed to parse TOML from {source}: {error}")
            }
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

    #[test]
    fn active_knowledge_uses_bundled_pack_in_this_release() {
        assert!(std::ptr::eq(active_knowledge(), bundled_knowledge()));
    }

    #[test]
    fn source_name_is_reported_for_parse_errors() {
        let source = KnowledgePackSource::new("test:broken", "not = [valid");
        let error = load_from_source(source).expect_err("invalid TOML must fail");

        assert!(
            error.to_string().contains("test:broken"),
            "error should include source name: {error}"
        );
    }

    #[test]
    fn bundled_knowledge_is_valid() {
        use crate::knowledge::validate::validate_knowledge_base;
        validate_knowledge_base(bundled_knowledge()).expect("bundled knowledge must validate");
    }

    #[test]
    fn all_languages_have_support_and_matchers() {
        for language in &bundled_knowledge().languages {
            assert!(
                !language.extensions.is_empty() || !language.filenames.is_empty(),
                "{} must be discoverable",
                language.id
            );
        }
    }

    #[test]
    fn all_registered_rules_have_knowledge_applicability() {
        use crate::rules::registry::all_rule_metadata;
        use std::collections::HashSet;

        let known_rules = bundled_knowledge()
            .rule_applicability
            .iter()
            .map(|rule| rule.rule_id.as_str())
            .collect::<HashSet<_>>();

        for rule in all_rule_metadata() {
            assert!(
                known_rules.contains(rule.rule_id),
                "{} must have a knowledge applicability entry",
                rule.rule_id
            );
        }
    }

    #[test]
    fn rule_lifecycle_requires_metadata_knowledge_and_recommendation() {
        use crate::rules::registry::all_rule_metadata;
        use std::collections::HashSet;

        let known_rules = bundled_knowledge()
            .rule_applicability
            .iter()
            .map(|rule| rule.rule_id.as_str())
            .collect::<HashSet<_>>();

        for rule in all_rule_metadata() {
            assert!(
                known_rules.contains(rule.rule_id),
                "{} must have a Knowledge Engine applicability entry",
                rule.rule_id
            );
            assert!(
                rule.recommendation
                    .is_some_and(|recommendation| !recommendation.trim().is_empty()),
                "{} must have a user-facing recommendation",
                rule.rule_id
            );
        }
    }
}

