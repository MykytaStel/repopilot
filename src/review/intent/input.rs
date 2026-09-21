use super::{IntentContract, IntentError};
use crate::config::model::RepoPilotConfig;
use crate::path_security::RootConfinement;
use crate::review::contract::ContractFamily;
use globset::Glob;
use serde::Deserialize;
use serde_json::Value;
use std::collections::BTreeSet;
use std::fs;
use std::path::Path;

pub const MAX_INTENT_BYTES: usize = 16 * 1024;
const MAX_LIST_ENTRIES: usize = 32;
const MAX_ENTRY_BYTES: usize = 256;
const MAX_SUMMARY_BYTES: usize = 256;

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct IntentWire {
    version: u16,
    #[serde(default)]
    summary: Option<String>,
    #[serde(default)]
    paths: Vec<String>,
    #[serde(default)]
    contract_families: Vec<String>,
    #[serde(default)]
    critical_paths: Vec<String>,
    #[serde(default)]
    verification: Vec<String>,
}

pub fn parse_intent_toml(contents: &str) -> Result<IntentContract, IntentError> {
    check_size(contents.len(), "intent document")?;
    let wire: IntentWire = toml::from_str(contents)
        .map_err(|error| IntentError::new(format!("invalid intent document: {error}")))?;
    from_wire(wire)
}

pub fn parse_intent_json(value: &Value) -> Result<IntentContract, IntentError> {
    let encoded = serde_json::to_vec(value)
        .map_err(|error| IntentError::new(format!("intent JSON could not be encoded: {error}")))?;
    check_size(encoded.len(), "intent JSON")?;
    let wire: IntentWire = serde_json::from_value(value.clone())
        .map_err(|error| IntentError::new(format!("invalid intent JSON: {error}")))?;
    from_wire(wire)
}

pub fn load_intent_file(
    root: &Path,
    path: &Path,
    config: &RepoPilotConfig,
) -> Result<IntentContract, IntentError> {
    let confinement = RootConfinement::named(root, "repository root").map_err(IntentError::new)?;
    let resolved = confinement
        .resolve_existing(path, "intent file")
        .map_err(IntentError::new)?;
    let metadata = fs::metadata(&resolved)
        .map_err(|error| IntentError::new(format!("intent file unavailable: {error}")))?;
    check_size(
        usize::try_from(metadata.len()).unwrap_or(usize::MAX),
        "intent file",
    )?;
    let contents = fs::read_to_string(&resolved)
        .map_err(|error| IntentError::new(format!("intent file is not valid UTF-8: {error}")))?;
    let intent = parse_intent_toml(&contents)?;
    validate_intent(&intent, config)?;
    Ok(intent)
}

pub fn validate_intent(
    intent: &IntentContract,
    config: &RepoPilotConfig,
) -> Result<(), IntentError> {
    if intent.version != 1 {
        return Err(IntentError::new(format!(
            "unsupported intent version {}; expected 1",
            intent.version
        )));
    }
    if intent.paths.is_empty()
        && intent.contract_families.is_empty()
        && intent.critical_paths.is_empty()
        && intent.verification.is_empty()
    {
        return Err(IntentError::new(
            "intent must declare at least one bounded scope or verification ID",
        ));
    }
    for path in &intent.paths {
        validate_relative_pattern(path, "intent path")?;
    }
    validate_critical_paths(config)?;
    let known_critical = config
        .review
        .critical_paths
        .iter()
        .map(|rule| rule.name.as_str())
        .collect::<BTreeSet<_>>();
    for name in &intent.critical_paths {
        if !known_critical.contains(name.as_str()) {
            return Err(IntentError::new(format!(
                "intent references unknown critical path `{name}`"
            )));
        }
    }
    let known_checks = config
        .verification
        .checks
        .iter()
        .map(|check| check.id.as_str())
        .collect::<BTreeSet<_>>();
    for id in &intent.verification {
        if !known_checks.contains(id.as_str()) {
            return Err(IntentError::new(format!(
                "intent references unknown verification ID `{id}`"
            )));
        }
    }
    Ok(())
}

pub fn validate_critical_paths(config: &RepoPilotConfig) -> Result<(), IntentError> {
    if config.review.critical_paths.len() > MAX_LIST_ENTRIES {
        return Err(IntentError::new(format!(
            "configured critical paths exceed the {MAX_LIST_ENTRIES}-entry limit"
        )));
    }
    let mut names = BTreeSet::new();
    for rule in &config.review.critical_paths {
        if rule.name.trim().is_empty() {
            return Err(IntentError::new("critical path name cannot be empty"));
        }
        check_field(&rule.name, MAX_ENTRY_BYTES, "critical path name")?;
        if !names.insert(rule.name.as_str()) {
            return Err(IntentError::new(format!(
                "critical path `{}` is declared more than once",
                rule.name
            )));
        }
        if rule.paths.is_empty() {
            return Err(IntentError::new(format!(
                "critical path `{}` must declare at least one path",
                rule.name
            )));
        }
        for path in &rule.paths {
            validate_relative_pattern(path, "critical path")?;
        }
    }
    check_list(
        &config
            .review
            .critical_paths
            .iter()
            .flat_map(|rule| rule.paths.iter().cloned())
            .collect::<Vec<_>>(),
        "critical path patterns",
    )?;
    Ok(())
}

fn from_wire(wire: IntentWire) -> Result<IntentContract, IntentError> {
    if wire.version != 1 {
        return Err(IntentError::new(format!(
            "unsupported intent version {}; expected 1",
            wire.version
        )));
    }
    if let Some(summary) = &wire.summary {
        check_field(summary, MAX_SUMMARY_BYTES, "intent summary")?;
    }
    check_list(&wire.paths, "intent paths")?;
    check_list(&wire.contract_families, "intent contract families")?;
    check_list(&wire.critical_paths, "intent critical paths")?;
    check_list(&wire.verification, "intent verification")?;
    let contract_families = wire
        .contract_families
        .iter()
        .map(|family| parse_family(family))
        .collect::<Result<Vec<_>, _>>()?;
    Ok(IntentContract {
        version: wire.version,
        summary: wire.summary,
        paths: sorted_strings(&wire.paths),
        contract_families: sorted_unique(&contract_families),
        critical_paths: sorted_strings(&wire.critical_paths),
        verification: sorted_strings(&wire.verification),
    })
}

fn parse_family(value: &str) -> Result<ContractFamily, IntentError> {
    match value {
        "public-symbol" => Ok(ContractFamily::PublicSymbol),
        "dependency" => Ok(ContractFamily::Dependency),
        "delivery" => Ok(ContractFamily::Delivery),
        "runtime-configuration" => Ok(ContractFamily::RuntimeConfiguration),
        "security-boundary" => Ok(ContractFamily::SecurityBoundary),
        "test-coverage" => Ok(ContractFamily::TestCoverage),
        _ => Err(IntentError::new(format!(
            "unknown contract family `{value}`"
        ))),
    }
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

fn check_size(size: usize, label: &str) -> Result<(), IntentError> {
    if size > MAX_INTENT_BYTES {
        return Err(IntentError::new(format!(
            "{label} exceeds the {MAX_INTENT_BYTES}-byte limit"
        )));
    }
    Ok(())
}

fn check_list(values: &[String], label: &str) -> Result<(), IntentError> {
    if values.len() > MAX_LIST_ENTRIES {
        return Err(IntentError::new(format!(
            "{label} exceeds the {MAX_LIST_ENTRIES}-entry limit"
        )));
    }
    for value in values {
        check_field(value, MAX_ENTRY_BYTES, label)?;
    }
    Ok(())
}

fn check_field(value: &str, limit: usize, label: &str) -> Result<(), IntentError> {
    if value.len() > limit {
        return Err(IntentError::new(format!(
            "{label} entry exceeds the {limit}-byte limit"
        )));
    }
    Ok(())
}

fn validate_relative_pattern(value: &str, label: &str) -> Result<(), IntentError> {
    let normalized = value.replace('\\', "/");
    let path = Path::new(&normalized);
    if normalized.is_empty()
        || path.is_absolute()
        || normalized.contains(':')
        || normalized.split('/').any(|part| part == "..")
    {
        return Err(IntentError::new(format!(
            "{label} `{value}` must be repository-relative and cannot escape the root"
        )));
    }
    Glob::new(&normalized).map_err(|error| {
        IntentError::new(format!("{label} `{value}` is not a valid glob: {error}"))
    })?;
    Ok(())
}
