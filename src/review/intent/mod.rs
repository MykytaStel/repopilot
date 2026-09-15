//! Bounded, deterministic intent comparison for Phase C.
//!
//! Intent is data, not executable policy. It can describe expected path and
//! contract boundaries, but it never supplies a command or suppresses proof.

use crate::config::model::CriticalPathRule;
use crate::review::contract::ContractFamily;
use globset::Glob;
use serde::Serialize;
use std::fmt;

mod input;
pub use input::validate_critical_paths;
pub use input::{load_intent_file, parse_intent_json, parse_intent_toml, validate_intent};

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct IntentContract {
    pub version: u16,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub summary: Option<String>,
    pub paths: Vec<String>,
    pub contract_families: Vec<ContractFamily>,
    pub critical_paths: Vec<String>,
    pub verification: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct IntentError(pub(crate) String);

impl IntentError {
    pub(crate) fn new(message: impl Into<String>) -> Self {
        Self(message.into())
    }
}

impl fmt::Display for IntentError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(&self.0)
    }
}

impl std::error::Error for IntentError {}

/// Inputs owned by the review pipeline. Keeping the optional contract and
/// repository policy together lets every output derive one canonical
/// assessment without a renderer-specific decision path.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct IntentContext {
    pub contract: Option<IntentContract>,
    pub critical_paths: Vec<CriticalPathRule>,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum IntentStatus {
    #[default]
    NotSupplied,
    WithinScope,
    Drifted,
}

impl IntentStatus {
    pub fn label(self) -> &'static str {
        match self {
            Self::NotSupplied => "not-supplied",
            Self::WithinScope => "within-scope",
            Self::Drifted => "drifted",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct CriticalPathMatch {
    pub name: String,
    pub paths: Vec<String>,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize)]
pub struct IntentDrift {
    pub status: IntentStatus,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub summary: Option<String>,
    pub declared_paths: Vec<String>,
    pub declared_contract_families: Vec<ContractFamily>,
    pub declared_critical_paths: Vec<String>,
    pub expected_verification: Vec<String>,
    pub actual_paths: Vec<String>,
    pub actual_contract_families: Vec<ContractFamily>,
    pub critical_path_matches: Vec<CriticalPathMatch>,
    pub unexpected_paths: Vec<String>,
    pub unexpected_contract_families: Vec<ContractFamily>,
    pub unexpected_critical_paths: Vec<String>,
    pub missing_verification: Vec<String>,
}

impl IntentDrift {
    pub fn is_drifted(&self) -> bool {
        self.status == IntentStatus::Drifted
    }
}

pub fn evaluate_intent(
    intent: Option<&IntentContract>,
    actual_paths: &[String],
    actual_contract_families: &[ContractFamily],
    critical_paths: &[CriticalPathRule],
    selected_verification: &[String],
) -> IntentDrift {
    let actual_paths = sorted_strings(actual_paths);
    let actual_contract_families = sorted_unique(actual_contract_families);
    let critical_path_matches = critical_matches(&actual_paths, critical_paths);
    let Some(intent) = intent else {
        return IntentDrift {
            status: IntentStatus::NotSupplied,
            actual_paths,
            actual_contract_families,
            critical_path_matches,
            ..Default::default()
        };
    };

    let declared_paths = sorted_strings(&intent.paths);
    let declared_contract_families = sorted_unique(&intent.contract_families);
    let declared_critical_paths = sorted_strings(&intent.critical_paths);
    let expected_verification = sorted_strings(&intent.verification);
    let selected_verification = sorted_strings(selected_verification);
    let unexpected_paths = if declared_paths.is_empty() {
        Vec::new()
    } else {
        actual_paths
            .iter()
            .filter(|path| !matches_any(path, &declared_paths))
            .cloned()
            .collect()
    };
    let unexpected_contract_families = actual_contract_families
        .iter()
        .filter(|family| !declared_contract_families.contains(family))
        .copied()
        .collect::<Vec<_>>();
    let unexpected_critical_paths = critical_path_matches
        .iter()
        .filter(|matched| !declared_critical_paths.contains(&matched.name))
        .map(|matched| matched.name.clone())
        .collect::<Vec<_>>();
    let missing_verification = expected_verification
        .iter()
        .filter(|id| !selected_verification.contains(id))
        .cloned()
        .collect::<Vec<_>>();
    let drifted = !unexpected_paths.is_empty()
        || !unexpected_contract_families.is_empty()
        || !unexpected_critical_paths.is_empty()
        || !missing_verification.is_empty();
    IntentDrift {
        status: if drifted {
            IntentStatus::Drifted
        } else {
            IntentStatus::WithinScope
        },
        summary: intent.summary.clone(),
        declared_paths,
        declared_contract_families,
        declared_critical_paths,
        expected_verification,
        actual_paths,
        actual_contract_families,
        critical_path_matches,
        unexpected_paths,
        unexpected_contract_families,
        unexpected_critical_paths,
        missing_verification,
    }
}

fn critical_matches(
    actual_paths: &[String],
    critical_paths: &[CriticalPathRule],
) -> Vec<CriticalPathMatch> {
    let mut rules = critical_paths.iter().collect::<Vec<_>>();
    rules.sort_by(|left, right| left.name.cmp(&right.name));
    rules
        .into_iter()
        .filter_map(|rule| {
            let paths = actual_paths
                .iter()
                .filter(|path| matches_any_pattern(path, &rule.paths))
                .cloned()
                .collect::<Vec<_>>();
            (!paths.is_empty()).then(|| CriticalPathMatch {
                name: rule.name.clone(),
                paths,
            })
        })
        .collect()
}

fn matches_any(path: &str, patterns: &[String]) -> bool {
    patterns.iter().any(|pattern| {
        Glob::new(pattern)
            .map(|glob| glob.compile_matcher().is_match(path))
            .unwrap_or(false)
    })
}

fn matches_any_pattern(path: &str, patterns: &[String]) -> bool {
    matches_any(path, patterns)
}

fn sorted_strings(values: &[String]) -> Vec<String> {
    let mut values = values
        .iter()
        .map(|value| value.replace('\\', "/"))
        .collect::<Vec<_>>();
    values.sort();
    values.dedup();
    values
}

fn sorted_unique<T: Ord + Copy>(values: &[T]) -> Vec<T> {
    let mut values = values.to_vec();
    values.sort();
    values.dedup();
    values
}
