//! Conservative attribution for error handlers moved between changed files.

use super::removed_ast::{self, ErrorHandlerFingerprint};
use crate::audits::context::classify::helpers::is_test_file;
use crate::languages::removed_for_extension;
use crate::review::diff::{ChangeStatus, ChangedFile};
use crate::review::signals::content::ReviewSource;
use std::collections::BTreeMap;
use std::path::PathBuf;

pub(crate) struct ChangedBehaviorSource<'a> {
    pub(crate) file: &'a ChangedFile,
    pub(crate) pre: Option<&'a ReviewSource>,
    pub(crate) post: Option<&'a ReviewSource>,
}

/// Returns the number of removed handlers that have one unambiguous counterpart
/// in another changed file. Only clean ASTs, identical handler tokens (or the
/// same Rust Result-producing operation), unique source/destination files, and
/// equal occurrence counts qualify.
pub(crate) fn moved_error_handler_counts(
    changes: &[ChangedBehaviorSource<'_>],
) -> BTreeMap<PathBuf, usize> {
    if changes.len() < 2
        || !changes.iter().any(|change| {
            change
                .file
                .hunks
                .iter()
                .flat_map(|hunk| &hunk.removed_lines)
                .any(|line| looks_like_error_handler_line(line))
        })
        || (!changes.iter().any(|change| {
            change
                .file
                .hunks
                .iter()
                .flat_map(|hunk| &hunk.added_lines)
                .any(|line| looks_like_error_handler_line(line))
        }) && !changes.iter().any(|change| {
            matches!(
                change.file.status,
                ChangeStatus::Added | ChangeStatus::Untracked
            ) && change
                .post
                .is_some_and(|source| source.content().lines().any(looks_like_error_handler_line))
        }))
    {
        return BTreeMap::new();
    }

    let mut removed = BTreeMap::<ErrorHandlerFingerprint, BTreeMap<usize, usize>>::new();
    let mut added = BTreeMap::<ErrorHandlerFingerprint, BTreeMap<usize, usize>>::new();

    for (index, change) in changes.iter().enumerate() {
        if is_test_file(&change.file.path) {
            continue;
        }
        let Some(ext) = change.file.path.extension().and_then(|ext| ext.to_str()) else {
            continue;
        };
        let Some(tables) = removed_for_extension(ext) else {
            continue;
        };

        let before = change.pre.map_or_else(BTreeMap::new, |source| {
            removed_ast::changed_error_handler_fingerprints(source, change.file, tables, true)
        });
        let after = change.post.map_or_else(BTreeMap::new, |source| {
            removed_ast::changed_error_handler_fingerprints(source, change.file, tables, false)
        });

        for (fingerprint, count) in &before {
            let removed_count = count.saturating_sub(after.get(fingerprint).copied().unwrap_or(0));
            if removed_count > 0 {
                removed
                    .entry(fingerprint.clone())
                    .or_default()
                    .insert(index, removed_count);
            }
        }
        for (fingerprint, count) in &after {
            let added_count = count.saturating_sub(before.get(fingerprint).copied().unwrap_or(0));
            if added_count > 0 {
                added
                    .entry(fingerprint.clone())
                    .or_default()
                    .insert(index, added_count);
            }
        }
    }

    let mut moved_by_index = BTreeMap::<usize, usize>::new();
    for (fingerprint, removed_files) in &removed {
        let Some(added_files) = added.get(fingerprint) else {
            continue;
        };
        if removed_files.len() != 1 || added_files.len() != 1 {
            continue;
        }
        let Some((&removed_index, &removed_count)) = removed_files.iter().next() else {
            continue;
        };
        let Some((&added_index, &added_count)) = added_files.iter().next() else {
            continue;
        };
        if removed_index != added_index && removed_count == added_count {
            *moved_by_index.entry(removed_index).or_default() += removed_count;
        }
    }

    moved_by_index
        .into_iter()
        .map(|(index, count)| (changes[index].file.path.clone(), count))
        .collect()
}

fn looks_like_error_handler_line(line: &str) -> bool {
    let lower = line.to_ascii_lowercase();
    lower
        .split(|character: char| !character.is_ascii_alphanumeric())
        .any(|token| {
            matches!(
                token,
                "try" | "catch" | "except" | "finally" | "rescue" | "err"
            )
        })
        || lower.contains("map_err")
        || lower.contains("unwrap_or")
        || lower.contains('?')
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::review::diff::{ChangedRange, DiffHunk};

    #[test]
    fn matches_rust_match_to_question_mark_when_result_operation_is_preserved() {
        let before = ReviewSource::new(
            "fn load() -> Result<Value, Error> {\n    match parse_value() {\n        Ok(value) => Ok(value),\n        Err(error) => Err(error),\n    }\n}\n".to_string(),
            Some("Rust".to_string()),
        );
        let after = ReviewSource::new(
            "fn load() -> Result<Value, Error> {\n    let value = parse_value()?;\n    Ok(value)\n}\n".to_string(),
            Some("Rust".to_string()),
        );
        let old_file = ChangedFile {
            path: PathBuf::from("src/old.rs"),
            status: ChangeStatus::Modified,
            ranges: Vec::new(),
            hunks: vec![DiffHunk {
                header: None,
                new_range: None,
                old_range: Some(ChangedRange { start: 2, end: 5 }),
                added_lines: Vec::new(),
                removed_lines: vec![
                    "    match parse_value() {".to_string(),
                    "        Ok(value) => Ok(value),".to_string(),
                    "        Err(error) => Err(error),".to_string(),
                    "    }".to_string(),
                ],
            }],
        };
        let new_file = ChangedFile {
            path: PathBuf::from("src/new.rs"),
            status: ChangeStatus::Added,
            ranges: vec![ChangedRange { start: 1, end: 5 }],
            hunks: vec![DiffHunk {
                header: None,
                new_range: Some(ChangedRange { start: 1, end: 5 }),
                old_range: None,
                added_lines: after.content().lines().map(str::to_string).collect(),
                removed_lines: Vec::new(),
            }],
        };

        let exact_move = moved_error_handler_counts(&[
            ChangedBehaviorSource {
                file: &old_file,
                pre: Some(&before),
                post: None,
            },
            ChangedBehaviorSource {
                file: &new_file,
                pre: None,
                post: Some(&after),
            },
        ]);
        assert_eq!(exact_move.get(&old_file.path), Some(&1));

        let changed_after = ReviewSource::new(
            "fn load() -> Result<Value, Error> {\n    let value = parse_with_defaults()?;\n    Ok(value)\n}\n".to_string(),
            Some("Rust".to_string()),
        );
        let changed_file = ChangedFile {
            path: new_file.path.clone(),
            status: ChangeStatus::Added,
            ranges: new_file.ranges.clone(),
            hunks: vec![DiffHunk {
                added_lines: changed_after
                    .content()
                    .lines()
                    .map(str::to_string)
                    .collect(),
                ..new_file.hunks[0].clone()
            }],
        };
        let changed_operation = moved_error_handler_counts(&[
            ChangedBehaviorSource {
                file: &old_file,
                pre: Some(&before),
                post: None,
            },
            ChangedBehaviorSource {
                file: &changed_file,
                pre: None,
                post: Some(&changed_after),
            },
        ]);
        assert!(changed_operation.is_empty());
    }
}
