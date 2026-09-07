use crate::review::diff::ChangedFile;

use super::*;

#[path = "dependency/lockfile.rs"]
mod lockfile;
#[path = "dependency/parse.rs"]
mod parse;
use parse::*;

pub(super) fn dependency_deltas(changed_files: &[ChangedFile]) -> Vec<ChangeProofContractDelta> {
    let mut deltas = Vec::new();
    for file in changed_files {
        let Some(kind) = manifest_kind(&file.path) else {
            continue;
        };
        let path = file.path_string();
        if kind == ManifestKind::Lockfile {
            deltas.extend(lockfile::deltas(file, &path));
            continue;
        }

        for hunk in &file.hunks {
            let removed = hunk
                .removed_lines
                .iter()
                .filter_map(|line| {
                    parse_dependency_edit(kind, line, hunk.old_range.map(|r| r.start))
                })
                .collect::<Vec<_>>();
            let added = hunk
                .added_lines
                .iter()
                .filter_map(|line| {
                    parse_dependency_edit(kind, line, hunk.new_range.map(|r| r.start))
                })
                .collect::<Vec<_>>();
            let mut paired = Vec::new();
            for old in &removed {
                if let Some(index) = added.iter().position(|new| new.name == old.name) {
                    paired.push((old.clone(), added[index].clone()));
                }
            }
            for (old, new) in paired {
                let alias = is_workspace_alias(&old.specification)
                    || is_workspace_alias(&new.specification);
                let change = if alias {
                    ContractChangeKind::AliasChanged
                } else {
                    classify_pair(&old.specification, &new.specification)
                };
                let evidence = if alias {
                    format!(
                        "Workspace dependency alias `{}` changed from `{}` to `{}`; exact workspace resolution is not claimed.",
                        new.name, old.specification, new.specification
                    )
                } else {
                    format!(
                        "{} dependency `{}` changed from `{}` to `{}`.",
                        manifest_label(kind),
                        new.name,
                        old.specification,
                        new.specification
                    )
                };
                deltas.push(delta(
                    &path,
                    &new.name,
                    change,
                    new.line.or(old.line),
                    &evidence,
                    ContractConfidence::High,
                ));
            }
            for edit in removed
                .iter()
                .filter(|old| !added.iter().any(|new| new.name == old.name))
            {
                deltas.push(delta(
                    &path,
                    &edit.name,
                    ContractChangeKind::Removed,
                    edit.line,
                    &format!(
                        "{} dependency `{}` was removed.",
                        manifest_label(kind),
                        edit.name
                    ),
                    ContractConfidence::High,
                ));
            }
            for edit in added
                .iter()
                .filter(|new| !removed.iter().any(|old| old.name == new.name))
            {
                deltas.push(delta(
                    &path,
                    &edit.name,
                    ContractChangeKind::Added,
                    edit.line,
                    &format!(
                        "{} dependency `{}` was added.",
                        manifest_label(kind),
                        edit.name
                    ),
                    ContractConfidence::High,
                ));
            }
            if hunk
                .added_lines
                .iter()
                .chain(&hunk.removed_lines)
                .any(|line| is_metadata_line(kind, line))
            {
                deltas.push(delta(
                    &path,
                    "manifest metadata",
                    ContractChangeKind::MetadataOnly,
                    hunk.new_range.or(hunk.old_range).map(|range| range.start),
                    "Manifest metadata changed without a recognized dependency transition.",
                    ContractConfidence::High,
                ));
            }
        }
    }
    deltas.sort_by(|left, right| {
        left.exporter_path
            .cmp(&right.exporter_path)
            .then(left.consumer_path.cmp(&right.consumer_path))
            .then(left.line_start.cmp(&right.line_start))
            .then(left.change.cmp(&right.change))
    });
    deltas
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::review::diff::{ChangeStatus, ChangedRange, DiffHunk};
    use std::path::PathBuf;

    fn changed(path: &str, added: &[&str], removed: &[&str]) -> ChangedFile {
        changed_with_header(path, added, removed, None)
    }

    fn changed_with_header(
        path: &str,
        added: &[&str],
        removed: &[&str],
        header: Option<&str>,
    ) -> ChangedFile {
        ChangedFile {
            path: PathBuf::from(path),
            status: ChangeStatus::Modified,
            ranges: vec![ChangedRange { start: 4, end: 4 }],
            hunks: vec![DiffHunk {
                header: header.map(str::to_string),
                new_range: Some(ChangedRange { start: 4, end: 4 }),
                old_range: Some(ChangedRange { start: 4, end: 4 }),
                added_lines: added.iter().map(|line| (*line).to_string()).collect(),
                removed_lines: removed.iter().map(|line| (*line).to_string()).collect(),
            }],
        }
    }

    #[test]
    fn cargo_dependency_version_change_is_upgraded() {
        let deltas = dependency_deltas(&[changed(
            "Cargo.toml",
            &["serde = { version = \"1.0.210\", features = [\"derive\"] }"],
            &["serde = { version = \"1.0.200\", features = [\"derive\"] }"],
        )]);

        assert_eq!(deltas.len(), 1);
        assert_eq!(deltas[0].family, ContractFamily::Dependency);
        assert_eq!(deltas[0].change, ContractChangeKind::Upgraded);
        assert_eq!(deltas[0].consumer_path, "serde");
    }

    #[test]
    fn cargo_dependency_source_and_feature_changes_are_typed() {
        let source = dependency_deltas(&[changed(
            "Cargo.toml",
            &["thing = { git = \"https://example.invalid/new.git\" }"],
            &["thing = { git = \"https://example.invalid/old.git\" }"],
        )]);
        let feature = dependency_deltas(&[changed(
            "Cargo.toml",
            &["thing = { version = \"1.0\", features = [\"serde\"] }"],
            &["thing = { version = \"1.0\" }"],
        )]);

        assert_eq!(source[0].change, ContractChangeKind::SourceChanged);
        assert_eq!(feature[0].change, ContractChangeKind::FeatureChanged);
    }

    #[test]
    fn npm_dependency_addition_and_metadata_are_separate() {
        let deltas = dependency_deltas(&[changed(
            "package.json",
            &[
                "    \"react\": \"^19.0.0\",",
                "    \"description\": \"new\",",
            ],
            &["    \"description\": \"old\","],
        )]);

        assert!(deltas.iter().any(
            |delta| delta.consumer_path == "react" && delta.change == ContractChangeKind::Added
        ));
        assert!(deltas.iter().any(|delta| {
            delta.consumer_path == "manifest metadata"
                && delta.change == ContractChangeKind::MetadataOnly
        }));
    }

    #[test]
    fn lockfile_change_is_limited_without_package_identity() {
        let deltas = dependency_deltas(&[changed(
            "Cargo.lock",
            &["version = \"1.1.0\""],
            &["version = \"1.0.0\""],
        )]);

        assert_eq!(deltas.len(), 1);
        assert_eq!(deltas[0].change, ContractChangeKind::MetadataOnly);
        assert_eq!(deltas[0].confidence, Some(ContractConfidence::Limited));

        let npm_deltas = dependency_deltas(&[changed(
            "package-lock.json",
            &["    \"version\": \"2.0.0\","],
            &["    \"version\": \"1.0.0\","],
        )]);
        assert_eq!(npm_deltas.len(), 1);
        assert_eq!(npm_deltas[0].confidence, Some(ContractConfidence::Limited));
    }

    #[test]
    fn cargo_lockfile_version_change_keeps_package_identity() {
        let deltas = dependency_deltas(&[changed_with_header(
            "Cargo.lock",
            &["version = \"2.1.0\""],
            &["version = \"2.0.0\""],
            Some("name = \"beta\""),
        )]);

        assert_eq!(deltas.len(), 1);
        assert_eq!(deltas[0].consumer_path, "beta");
        assert_eq!(deltas[0].change, ContractChangeKind::Upgraded);
        assert_eq!(deltas[0].confidence, Some(ContractConfidence::High));
    }

    #[test]
    fn workspace_alias_transition_is_typed_for_cargo() {
        let deltas = dependency_deltas(&[changed(
            "crates/app/Cargo.toml",
            &["shared = { workspace = true }"],
            &["shared = { version = \"1.0\" }"],
        )]);

        assert_eq!(deltas.len(), 1);
        assert_eq!(deltas[0].change, ContractChangeKind::AliasChanged);
        assert!(deltas[0].evidence.contains("Workspace dependency alias"));
    }

    #[test]
    fn workspace_alias_transition_is_typed_for_npm() {
        let deltas = dependency_deltas(&[changed(
            "packages/app/package.json",
            &["    \"shared\": \"workspace:^\","],
            &["    \"shared\": \"workspace:*\","],
        )]);

        assert_eq!(deltas.len(), 1);
        assert_eq!(deltas[0].change, ContractChangeKind::AliasChanged);
    }
}
