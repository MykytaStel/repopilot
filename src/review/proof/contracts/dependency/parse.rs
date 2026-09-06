use super::super::{
    ChangeProofContractDelta, ContractChangeKind, ContractConfidence, ContractFamily,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum ManifestKind {
    Cargo,
    Npm,
    Lockfile,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct DependencyEdit {
    pub(super) name: String,
    pub(super) specification: String,
    pub(super) line: Option<usize>,
}

pub(super) fn manifest_kind(path: &std::path::Path) -> Option<ManifestKind> {
    match path.file_name()?.to_str()? {
        "Cargo.toml" => Some(ManifestKind::Cargo),
        "package.json" => Some(ManifestKind::Npm),
        "Cargo.lock" | "package-lock.json" | "pnpm-lock.yaml" | "yarn.lock" => {
            Some(ManifestKind::Lockfile)
        }
        _ => None,
    }
}

pub(super) fn parse_dependency_edit(
    kind: ManifestKind,
    line: &str,
    line_start: Option<usize>,
) -> Option<DependencyEdit> {
    let trimmed = line.trim();
    if kind == ManifestKind::Cargo {
        let (name, specification) = trimmed.split_once('=')?;
        let name = name.trim();
        let specification = specification.trim().trim_end_matches(',').to_string();
        if !is_cargo_dependency_name(name) || !looks_like_cargo_specification(&specification) {
            return None;
        }
        return Some(DependencyEdit {
            name: name.to_string(),
            specification,
            line: line_start,
        });
    }
    let trimmed = trimmed.strip_prefix('"')?;
    let (name, specification) = trimmed.split_once("\":")?;
    let name = name.trim();
    let specification = specification
        .trim()
        .trim_end_matches(',')
        .trim_matches('"')
        .to_string();
    if !is_npm_dependency_name(name) || !looks_like_npm_specification(&specification) {
        return None;
    }
    Some(DependencyEdit {
        name: name.to_string(),
        specification,
        line: line_start,
    })
}

fn is_cargo_dependency_name(name: &str) -> bool {
    !name.is_empty()
        && name
            .chars()
            .all(|character| character.is_ascii_alphanumeric() || matches!(character, '_' | '-'))
        && !matches!(
            name,
            "name"
                | "version"
                | "edition"
                | "authors"
                | "description"
                | "license"
                | "repository"
                | "homepage"
                | "documentation"
                | "readme"
                | "build"
                | "rust-version"
                | "resolver"
                | "workspace"
                | "members"
                | "default-members"
                | "default"
                | "publish"
                | "links"
        )
}

fn is_npm_dependency_name(name: &str) -> bool {
    !name.is_empty()
        && !matches!(
            name,
            "name"
                | "version"
                | "description"
                | "private"
                | "scripts"
                | "main"
                | "module"
                | "types"
                | "type"
                | "license"
                | "repository"
                | "homepage"
                | "engines"
                | "files"
                | "keywords"
                | "author"
                | "contributors"
        )
        && (name.starts_with('@') || name.contains('-') || name.contains('/') || name.len() > 1)
}

fn looks_like_cargo_specification(specification: &str) -> bool {
    specification.starts_with('{')
        || specification == "*"
        || version_number(specification).is_some()
}

fn looks_like_npm_specification(specification: &str) -> bool {
    specification
        .chars()
        .next()
        .is_some_and(|character| matches!(character, '^' | '~' | '>' | '<' | '=' | '*'))
        || specification == "latest"
        || specification.starts_with("workspace:")
        || specification.starts_with("file:")
        || specification.starts_with("git")
        || version_number(specification).is_some()
}

pub(super) fn is_metadata_line(kind: ManifestKind, line: &str) -> bool {
    let trimmed = line.trim();
    match kind {
        ManifestKind::Cargo => {
            trimmed.starts_with("description =")
                || trimmed.starts_with("license =")
                || trimmed.starts_with("repository =")
                || trimmed.starts_with("homepage =")
                || trimmed.starts_with("readme =")
        }
        ManifestKind::Npm => {
            trimmed.starts_with("\"description\":")
                || trimmed.starts_with("\"homepage\":")
                || trimmed.starts_with("\"repository\":")
                || trimmed.starts_with("\"license\":")
                || trimmed.starts_with("\"private\":")
                || trimmed.starts_with("\"version\":")
                || trimmed.starts_with("\"scripts\":")
        }
        ManifestKind::Lockfile => false,
    }
}

pub(super) fn classify_pair(old: &str, new: &str) -> ContractChangeKind {
    if source_part(old) != source_part(new) {
        ContractChangeKind::SourceChanged
    } else if feature_part(old) != feature_part(new) {
        ContractChangeKind::FeatureChanged
    } else {
        match (version_number(old), version_number(new)) {
            (Some(old), Some(new)) if new > old => ContractChangeKind::Upgraded,
            (Some(old), Some(new)) if new < old => ContractChangeKind::Downgraded,
            _ if normalize_specification(old) == normalize_specification(new) => {
                ContractChangeKind::MetadataOnly
            }
            _ => ContractChangeKind::MetadataOnly,
        }
    }
}

fn source_part(specification: &str) -> String {
    specification
        .split([',', '}'])
        .filter(|part| {
            part.contains("git")
                || part.contains("path")
                || part.contains("workspace")
                || part.contains("file:")
                || part.contains("https://")
        })
        .collect::<Vec<_>>()
        .join(",")
}

fn feature_part(specification: &str) -> String {
    specification
        .split([',', '}', ']'])
        .filter(|part| part.contains("feature") || part.contains("optional"))
        .collect::<Vec<_>>()
        .join(",")
}

fn version_number(specification: &str) -> Option<(u64, u64, u64)> {
    let digits = specification
        .split(|character: char| !character.is_ascii_digit() && character != '.')
        .find(|part| !part.is_empty() && part.chars().any(|c| c.is_ascii_digit()))?;
    let mut parts = digits.split('.').map(|part| part.parse::<u64>().ok());
    Some((
        parts.next()??,
        parts.next()??,
        parts.next().unwrap_or(Some(0))?,
    ))
}

fn normalize_specification(specification: &str) -> String {
    specification
        .chars()
        .filter(|character| !character.is_whitespace())
        .collect()
}

pub(super) fn manifest_label(kind: ManifestKind) -> &'static str {
    match kind {
        ManifestKind::Cargo => "Cargo",
        ManifestKind::Npm => "npm",
        ManifestKind::Lockfile => "lockfile",
    }
}

pub(super) fn delta(
    path: &str,
    subject: &str,
    change: ContractChangeKind,
    line: Option<usize>,
    evidence: &str,
    confidence: ContractConfidence,
) -> ChangeProofContractDelta {
    ChangeProofContractDelta {
        family: ContractFamily::Dependency,
        change,
        exporter_path: path.to_string(),
        consumer_path: subject.to_string(),
        line_start: line,
        line_end: line,
        evidence: evidence.to_string(),
        confidence: Some(confidence),
    }
}
