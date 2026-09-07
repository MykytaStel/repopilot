use crate::review::diff::ChangedFile;
use std::path::Path;

use super::*;

#[path = "runtime/consumer.rs"]
mod consumer;

#[cfg(test)]
fn runtime_deltas(changed_files: &[ChangedFile]) -> Vec<ChangeProofContractDelta> {
    runtime_deltas_with_root(None, changed_files)
}

pub(super) fn runtime_deltas_in_repo(
    repo_root: &Path,
    changed_files: &[ChangedFile],
) -> Vec<ChangeProofContractDelta> {
    runtime_deltas_with_root(Some(repo_root), changed_files)
}

fn runtime_deltas_with_root(
    repo_root: Option<&Path>,
    changed_files: &[ChangedFile],
) -> Vec<ChangeProofContractDelta> {
    let config_edits = changed_files
        .iter()
        .filter(|file| is_runtime_config_path(&file.path_string()))
        .flat_map(|file| {
            file.hunks.iter().flat_map(move |hunk| {
                let removed = hunk
                    .removed_lines
                    .iter()
                    .filter_map(|line| parse_runtime_edit(line, hunk.old_range.map(|r| r.start)))
                    .map(|edit| (file.path_string(), false, edit));
                let added = hunk
                    .added_lines
                    .iter()
                    .filter_map(|line| parse_runtime_edit(line, hunk.new_range.map(|r| r.start)))
                    .map(|edit| (file.path_string(), true, edit));
                removed.chain(added).collect::<Vec<_>>()
            })
        })
        .collect::<Vec<_>>();
    let mut deltas = Vec::new();
    let mut consumed = Vec::new();
    for (index, (path, added, edit)) in config_edits.iter().enumerate() {
        if *added || consumed.contains(&index) {
            continue;
        }
        if let Some((other_index, (_, true, other))) =
            config_edits
                .iter()
                .enumerate()
                .find(|(other_index, (_, added, other))| {
                    !consumed.contains(other_index) && *added && other.key == edit.key
                })
        {
            consumed.extend([index, other_index]);
            deltas.push(runtime_delta(
                path,
                &edit.key,
                ContractChangeKind::Changed,
                edit.line,
                format!(
                    "Runtime key `{}` changed from `{}` to `{}`.",
                    edit.key, edit.value, other.value
                ),
                consumer::find(repo_root, changed_files, &edit.key),
            ));
            continue;
        }
        if let Some((other_index, (_, true, other))) =
            config_edits
                .iter()
                .enumerate()
                .find(|(other_index, (_, added, other))| {
                    !consumed.contains(other_index)
                        && *added
                        && other.key != edit.key
                        && other.value == edit.value
                })
        {
            consumed.extend([index, other_index]);
            deltas.push(runtime_delta(
                path,
                &other.key,
                ContractChangeKind::Renamed,
                other.line.or(edit.line),
                format!("Runtime key `{}` was renamed to `{}`.", edit.key, other.key),
                consumer::find(repo_root, changed_files, &other.key),
            ));
            continue;
        }
        consumed.push(index);
        deltas.push(runtime_delta(
            path,
            &edit.key,
            ContractChangeKind::Removed,
            edit.line,
            format!("Runtime key `{}` was removed.", edit.key),
            consumer::find(repo_root, changed_files, &edit.key),
        ));
    }
    for (index, (path, added, edit)) in config_edits.iter().enumerate() {
        if !*added || consumed.contains(&index) {
            continue;
        }
        consumed.push(index);
        deltas.push(runtime_delta(
            path,
            &edit.key,
            ContractChangeKind::Introduced,
            edit.line,
            format!("Runtime key `{}` was introduced.", edit.key),
            consumer::find(repo_root, changed_files, &edit.key),
        ));
    }
    deltas.sort_by(|left, right| {
        left.exporter_path
            .cmp(&right.exporter_path)
            .then(left.consumer_path.cmp(&right.consumer_path))
            .then(left.line_start.cmp(&right.line_start))
    });
    deltas
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct RuntimeEdit {
    key: String,
    value: String,
    line: Option<usize>,
}

fn is_runtime_config_path(path: &str) -> bool {
    let lower = path.to_ascii_lowercase();
    let file = lower.rsplit('/').next().unwrap_or(lower.as_str());
    file == ".env"
        || file.starts_with(".env.")
        || lower.contains("/config/")
        || lower.starts_with("config/")
}

fn parse_runtime_edit(line: &str, line_start: Option<usize>) -> Option<RuntimeEdit> {
    let trimmed = line.trim();
    let (key, value) = trimmed
        .split_once('=')
        .or_else(|| trimmed.split_once(':'))?;
    let key = key.trim().trim_matches('"');
    if !is_runtime_key(key) {
        return None;
    }
    Some(RuntimeEdit {
        key: key.to_string(),
        value: value
            .trim()
            .trim_end_matches(',')
            .trim_matches('"')
            .to_string(),
        line: line_start,
    })
}

fn is_runtime_key(key: &str) -> bool {
    !key.is_empty()
        && key
            .chars()
            .all(|character| character.is_ascii_alphanumeric() || matches!(character, '_' | '-'))
        && key.chars().any(|character| character.is_ascii_uppercase())
}

fn runtime_delta(
    path: &str,
    key: &str,
    change: ContractChangeKind,
    line: Option<usize>,
    evidence: String,
    consumer: Option<consumer::ConsumerMatch>,
) -> ChangeProofContractDelta {
    let (consumer_path, confidence, evidence) = match consumer {
        Some(match_) => (
            match_.path,
            match_.confidence,
            format!("{evidence} {}", match_.evidence),
        ),
        None => (key.to_string(), ContractConfidence::Limited, evidence),
    };
    ChangeProofContractDelta {
        family: ContractFamily::RuntimeConfiguration,
        change,
        exporter_path: path.to_string(),
        consumer_path,
        line_start: line,
        line_end: line,
        evidence,
        confidence: Some(confidence),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::review::diff::{ChangeStatus, ChangedRange, DiffHunk};
    use std::path::PathBuf;

    fn changed(path: &str, added: &[&str], removed: &[&str]) -> ChangedFile {
        ChangedFile {
            path: PathBuf::from(path),
            status: ChangeStatus::Modified,
            ranges: vec![ChangedRange { start: 4, end: 4 }],
            hunks: vec![DiffHunk {
                header: None,
                new_range: Some(ChangedRange { start: 4, end: 4 }),
                old_range: Some(ChangedRange { start: 4, end: 4 }),
                added_lines: added.iter().map(|line| (*line).to_string()).collect(),
                removed_lines: removed.iter().map(|line| (*line).to_string()).collect(),
            }],
        }
    }

    #[test]
    fn introduced_runtime_key_requires_a_changed_consumer_for_high_confidence() {
        let deltas = runtime_deltas(&[
            changed(".env", &["DATABASE_URL=postgres://new"], &[]),
            changed(
                "src/config.ts",
                &["const url = process.env.DATABASE_URL"],
                &[],
            ),
        ]);

        assert_eq!(deltas.len(), 1);
        assert_eq!(deltas[0].family, ContractFamily::RuntimeConfiguration);
        assert_eq!(deltas[0].change, ContractChangeKind::Introduced);
        assert_eq!(deltas[0].confidence, Some(ContractConfidence::High));
    }

    #[test]
    fn removed_runtime_key_without_a_consumer_is_limited() {
        let deltas = runtime_deltas(&[changed(".env", &[], &["OLD_URL=https://old"])]);

        assert_eq!(deltas.len(), 1);
        assert_eq!(deltas[0].change, ContractChangeKind::Removed);
        assert_eq!(deltas[0].confidence, Some(ContractConfidence::Limited));
    }

    #[test]
    fn paired_runtime_key_values_can_be_classified_as_renamed() {
        let deltas = runtime_deltas(&[changed(
            ".env",
            &["SERVICE_URL=https://example"],
            &["API_URL=https://example"],
        )]);

        assert_eq!(deltas.len(), 1);
        assert_eq!(deltas[0].change, ContractChangeKind::Renamed);
    }

    #[test]
    fn unchanged_consumer_is_found_with_high_confidence() {
        let root = tempfile::tempdir().expect("temp root");
        std::fs::create_dir_all(root.path().join("src")).expect("src directory");
        std::fs::write(
            root.path().join("src/server.ts"),
            "const url = process.env.DATABASE_URL;\n",
        )
        .expect("consumer source");

        let deltas = runtime_deltas_in_repo(
            root.path(),
            &[changed(".env", &["DATABASE_URL=postgres://new"], &[])],
        );

        assert_eq!(deltas.len(), 1);
        assert_eq!(deltas[0].consumer_path, "src/server.ts");
        assert_eq!(deltas[0].confidence, Some(ContractConfidence::High));
        assert!(deltas[0].evidence.contains("unchanged consumer"));
    }

    #[test]
    fn ambiguous_unchanged_consumers_remain_limited() {
        let root = tempfile::tempdir().expect("temp root");
        std::fs::create_dir_all(root.path().join("src")).expect("src directory");
        for path in ["src/a.ts", "src/b.ts"] {
            std::fs::write(
                root.path().join(path),
                "const url = process.env.DATABASE_URL;\n",
            )
            .expect("consumer source");
        }

        let deltas = runtime_deltas_in_repo(
            root.path(),
            &[changed(".env", &["DATABASE_URL=postgres://new"], &[])],
        );

        assert_eq!(deltas.len(), 1);
        assert_eq!(deltas[0].confidence, Some(ContractConfidence::Limited));
        assert!(deltas[0].evidence.contains("multiple consumers"));
    }
}
