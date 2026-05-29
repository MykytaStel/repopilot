use crate::knowledge::model::KnowledgeBase;
use crate::rules::lookup_rule_metadata;
use std::collections::HashSet;

pub fn validate_knowledge_base(base: &KnowledgeBase) -> Result<(), KnowledgeValidationError> {
    validate_unique(
        "language",
        base.languages.iter().map(|language| language.id.as_str()),
    )?;
    validate_unique(
        "framework",
        base.frameworks
            .iter()
            .map(|framework| framework.id.as_str()),
    )?;
    validate_unique(
        "runtime",
        base.runtimes.iter().map(|runtime| runtime.id.as_str()),
    )?;
    validate_unique(
        "paradigm",
        base.paradigms.iter().map(|paradigm| paradigm.id.as_str()),
    )?;
    validate_unique(
        "rule applicability",
        base.rule_applicability
            .iter()
            .map(|rule| rule.rule_id.as_str()),
    )?;

    for language in &base.languages {
        if language.extensions.is_empty() && language.filenames.is_empty() {
            return Err(KnowledgeValidationError::MissingLanguageMatcher {
                language_id: language.id.clone(),
            });
        }
    }

    let language_ids = id_set(base.languages.iter().map(|language| language.id.as_str()));
    let framework_ids = id_set(
        base.frameworks
            .iter()
            .map(|framework| framework.id.as_str()),
    );
    let runtime_ids = id_set(base.runtimes.iter().map(|runtime| runtime.id.as_str()));
    let paradigm_ids = id_set(base.paradigms.iter().map(|paradigm| paradigm.id.as_str()));

    for rule in &base.rule_applicability {
        if lookup_rule_metadata(&rule.rule_id).is_none() {
            return Err(KnowledgeValidationError::UnknownRule {
                rule_id: rule.rule_id.clone(),
            });
        }

        validate_refs("language", &rule.rule_id, &rule.languages, &language_ids)?;
        validate_refs("framework", &rule.rule_id, &rule.frameworks, &framework_ids)?;
        validate_refs("runtime", &rule.rule_id, &rule.runtimes, &runtime_ids)?;
        validate_refs("paradigm", &rule.rule_id, &rule.paradigms, &paradigm_ids)?;

        for override_rule in &rule.overrides {
            if let Some(language) = &override_rule.language {
                validate_ref("language", &rule.rule_id, language, &language_ids)?;
            }
            if let Some(framework) = &override_rule.framework {
                validate_ref("framework", &rule.rule_id, framework, &framework_ids)?;
            }
            if let Some(runtime) = &override_rule.runtime {
                validate_ref("runtime", &rule.rule_id, runtime, &runtime_ids)?;
            }
            if let Some(paradigm) = &override_rule.paradigm {
                validate_ref("paradigm", &rule.rule_id, paradigm, &paradigm_ids)?;
            }
        }
    }

    Ok(())
}

fn validate_unique<'a>(
    kind: &'static str,
    ids: impl Iterator<Item = &'a str>,
) -> Result<(), KnowledgeValidationError> {
    let mut seen = HashSet::new();
    for id in ids {
        if !seen.insert(id) {
            return Err(KnowledgeValidationError::DuplicateId {
                kind,
                id: id.to_string(),
            });
        }
    }
    Ok(())
}

fn validate_refs(
    kind: &'static str,
    rule_id: &str,
    values: &HashSet<String>,
    known: &HashSet<String>,
) -> Result<(), KnowledgeValidationError> {
    for value in values {
        validate_ref(kind, rule_id, value, known)?;
    }
    Ok(())
}

fn validate_ref(
    kind: &'static str,
    rule_id: &str,
    value: &str,
    known: &HashSet<String>,
) -> Result<(), KnowledgeValidationError> {
    if known.contains(value) {
        Ok(())
    } else {
        Err(KnowledgeValidationError::UnknownReference {
            kind,
            rule_id: rule_id.to_string(),
            value: value.to_string(),
        })
    }
}

fn id_set<'a>(ids: impl Iterator<Item = &'a str>) -> HashSet<String> {
    ids.map(str::to_string).collect()
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum KnowledgeValidationError {
    DuplicateId {
        kind: &'static str,
        id: String,
    },
    MissingLanguageMatcher {
        language_id: String,
    },
    UnknownRule {
        rule_id: String,
    },
    UnknownReference {
        kind: &'static str,
        rule_id: String,
        value: String,
    },
}

impl std::fmt::Display for KnowledgeValidationError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            KnowledgeValidationError::DuplicateId { kind, id } => {
                write!(formatter, "duplicate {kind} id `{id}`")
            }
            KnowledgeValidationError::MissingLanguageMatcher { language_id } => {
                write!(
                    formatter,
                    "language `{language_id}` must have at least one extension or filename matcher"
                )
            }
            KnowledgeValidationError::UnknownRule { rule_id } => {
                write!(formatter, "knowledge references unknown rule `{rule_id}`")
            }
            KnowledgeValidationError::UnknownReference {
                kind,
                rule_id,
                value,
            } => write!(
                formatter,
                "rule `{rule_id}` references unknown {kind} `{value}`"
            ),
        }
    }
}

impl std::error::Error for KnowledgeValidationError {}
