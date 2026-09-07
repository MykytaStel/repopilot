use crate::review::diff::ChangedFile;

use super::*;

pub(super) fn delivery_deltas(changed_files: &[ChangedFile]) -> Vec<ChangeProofContractDelta> {
    let mut deltas = Vec::new();
    for file in changed_files {
        if !is_delivery_path(&file.path_string()) {
            continue;
        }
        let path = file.path_string();
        for hunk in &file.hunks {
            let removed = hunk
                .removed_lines
                .iter()
                .filter_map(|line| classify_line(line))
                .collect::<Vec<_>>();
            let added = hunk
                .added_lines
                .iter()
                .filter_map(|line| classify_line(line))
                .collect::<Vec<_>>();
            for kind in [
                DeliveryChange::Trigger,
                DeliveryChange::Permission,
                DeliveryChange::Secret,
                DeliveryChange::Action,
                DeliveryChange::Artifact,
                DeliveryChange::Deployment,
            ] {
                let old = removed.iter().find(|item| item.kind == kind);
                let new = added.iter().find(|item| item.kind == kind);
                let Some(current) = new.or(old) else {
                    continue;
                };
                let evidence = match (old, new) {
                    (Some(old), Some(new)) => format!(
                        "Delivery {} changed from `{}` to `{}`.",
                        kind.label(),
                        old.subject,
                        new.subject
                    ),
                    (None, Some(new)) => {
                        format!("Delivery {} introduced as `{}`.", kind.label(), new.subject)
                    }
                    (Some(old), None) => {
                        format!("Delivery {} removed from `{}`.", kind.label(), old.subject)
                    }
                    _ => unreachable!(),
                };
                deltas.push(delta(
                    &path,
                    &current.subject,
                    kind.change_kind(),
                    hunk.new_range.or(hunk.old_range).map(|range| range.start),
                    &evidence,
                ));
            }
        }
    }
    deltas.sort_by(|left, right| {
        left.exporter_path
            .cmp(&right.exporter_path)
            .then(left.consumer_path.cmp(&right.consumer_path))
            .then(left.line_start.cmp(&right.line_start))
    });
    deltas
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum DeliveryChange {
    Trigger,
    Permission,
    Secret,
    Action,
    Artifact,
    Deployment,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct DeliveryEdit {
    kind: DeliveryChange,
    subject: String,
}

impl DeliveryChange {
    fn label(self) -> &'static str {
        match self {
            Self::Trigger => "trigger",
            Self::Permission => "permission",
            Self::Secret => "secret use",
            Self::Action => "action reference",
            Self::Artifact => "artifact behavior",
            Self::Deployment => "deployment behavior",
        }
    }

    fn change_kind(self) -> ContractChangeKind {
        match self {
            Self::Trigger => ContractChangeKind::TriggerChanged,
            Self::Permission => ContractChangeKind::PermissionChanged,
            Self::Secret => ContractChangeKind::SecretUseChanged,
            Self::Action => ContractChangeKind::ActionReferenceChanged,
            Self::Artifact => ContractChangeKind::ArtifactChanged,
            Self::Deployment => ContractChangeKind::DeploymentChanged,
        }
    }
}

fn is_delivery_path(path: &str) -> bool {
    let lower = path.to_ascii_lowercase();
    lower.contains(".github/workflows/")
        || lower.ends_with("/action.yml")
        || lower.ends_with("/action.yaml")
        || lower == "action.yml"
        || lower == "action.yaml"
}

fn classify_line(line: &str) -> Option<DeliveryEdit> {
    let trimmed = line.trim();
    let normalized = trimmed.strip_prefix("- ").unwrap_or(trimmed);
    let lower = normalized.to_ascii_lowercase();
    let (kind, subject) = if lower.starts_with("permissions:") || lower.starts_with("permissions.")
    {
        (DeliveryChange::Permission, normalized.to_string())
    } else if lower.starts_with("on:") || lower.starts_with("\"on\":") {
        (DeliveryChange::Trigger, normalized.to_string())
    } else if lower.starts_with("uses:") {
        (
            DeliveryChange::Action,
            trimmed
                .split_once(':')
                .map(|(_, value)| value.trim().to_string())
                .unwrap_or_else(|| trimmed.to_string()),
        )
    } else if lower.starts_with("secrets:")
        || lower.contains("secrets.")
        || lower.contains("secretname")
    {
        (DeliveryChange::Secret, normalized.to_string())
    } else if lower.contains("upload-artifact")
        || lower.contains("download-artifact")
        || lower.contains("artifact")
    {
        (DeliveryChange::Artifact, normalized.to_string())
    } else if lower.contains("deploy")
        || lower.contains("kubectl")
        || lower.contains("helm ")
        || lower.contains("docker push")
        || lower.contains("aws ")
        || lower.contains("gcloud")
        || lower.contains("azure/login")
        || lower.starts_with("environment:")
    {
        (DeliveryChange::Deployment, normalized.to_string())
    } else {
        return None;
    };
    Some(DeliveryEdit { kind, subject })
}

fn delta(
    path: &str,
    subject: &str,
    change: ContractChangeKind,
    line: Option<usize>,
    evidence: &str,
) -> ChangeProofContractDelta {
    ChangeProofContractDelta {
        family: ContractFamily::Delivery,
        change,
        exporter_path: path.to_string(),
        consumer_path: subject.to_string(),
        line_start: line,
        line_end: line,
        evidence: evidence.to_string(),
        confidence: Some(ContractConfidence::High),
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
    fn workflow_permission_change_is_typed() {
        let deltas = delivery_deltas(&[changed(
            ".github/workflows/ci.yml",
            &["permissions: write-all"],
            &["permissions: read-all"],
        )]);

        assert_eq!(deltas.len(), 1);
        assert_eq!(deltas[0].family, ContractFamily::Delivery);
        assert_eq!(deltas[0].change, ContractChangeKind::PermissionChanged);
    }

    #[test]
    fn workflow_action_and_trigger_changes_are_typed() {
        let action = delivery_deltas(&[changed(
            ".github/workflows/ci.yml",
            &["- uses: actions/checkout@v5"],
            &["- uses: actions/checkout@v4"],
        )]);
        let trigger = delivery_deltas(&[changed(
            ".github/workflows/ci.yml",
            &["on: [push, pull_request]"],
            &["on: push"],
        )]);

        assert_eq!(action[0].change, ContractChangeKind::ActionReferenceChanged);
        assert_eq!(trigger[0].change, ContractChangeKind::TriggerChanged);
    }
}
