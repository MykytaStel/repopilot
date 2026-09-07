use crate::review::diff::ChangedFile;

use super::parse::{classify_pair, delta};
use super::{ChangeProofContractDelta, ContractChangeKind, ContractConfidence};

pub(super) fn deltas(file: &ChangedFile, path: &str) -> Vec<ChangeProofContractDelta> {
    let mut deltas = Vec::new();
    for hunk in &file.hunks {
        let old = hunk
            .removed_lines
            .iter()
            .find_map(|line| parse_lockfile_value(line, "version"));
        let new = hunk
            .added_lines
            .iter()
            .find_map(|line| parse_lockfile_value(line, "version"));
        let old_source = hunk
            .removed_lines
            .iter()
            .find_map(|line| parse_lockfile_value(line, "source"));
        let new_source = hunk
            .added_lines
            .iter()
            .find_map(|line| parse_lockfile_value(line, "source"));
        let subject = hunk.header.as_deref().and_then(lockfile_subject);
        let line = hunk.new_range.or(hunk.old_range).map(|range| range.start);
        let (change, evidence) = match (old, new, old_source, new_source) {
            (Some(old), Some(new), _, _) if subject.is_some() => (
                classify_pair(&old, &new),
                format!(
                    "Lockfile package `{}` changed version from `{old}` to `{new}`.",
                    subject.as_deref().unwrap_or("unknown")
                ),
            ),
            (Some(old), Some(new), _, _) => (
                ContractChangeKind::MetadataOnly,
                format!(
                    "Lockfile version changed from `{old}` to `{new}`, but package identity is unavailable."
                ),
            ),
            (_, _, Some(old), Some(new)) if subject.is_some() => (
                ContractChangeKind::SourceChanged,
                format!(
                    "Lockfile package `{}` changed source from `{old}` to `{new}`.",
                    subject.as_deref().unwrap_or("unknown")
                ),
            ),
            (_, _, Some(old), Some(new)) => (
                ContractChangeKind::MetadataOnly,
                format!(
                    "Lockfile source changed from `{old}` to `{new}`, but package identity is unavailable."
                ),
            ),
            _ => continue,
        };
        deltas.push(delta(
            path,
            subject.as_deref().unwrap_or("lockfile resolution"),
            change,
            line,
            &evidence,
            if subject.is_some() {
                ContractConfidence::High
            } else {
                ContractConfidence::Limited
            },
        ));
    }
    deltas
}

fn parse_lockfile_value(line: &str, key: &str) -> Option<String> {
    let trimmed = line.trim();
    if let Some((lhs, value)) = trimmed.split_once('=')
        && lhs.trim() == key
    {
        return Some(value.trim().trim_matches('"').to_string());
    }
    let json_key = format!("\"{key}\":");
    if let Some(value) = trimmed.strip_prefix(&json_key) {
        return Some(
            value
                .trim()
                .trim_end_matches(',')
                .trim_matches('"')
                .to_string(),
        );
    }
    let yaml_key = format!("{key}:");
    trimmed
        .strip_prefix(&yaml_key)
        .map(|value| value.trim().trim_matches('"').to_string())
}

fn lockfile_subject(header: &str) -> Option<String> {
    let value = header.strip_prefix("name = ")?;
    Some(value.trim().trim_matches('"').to_string())
}
