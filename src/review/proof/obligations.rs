use super::groups::{
    ProofObligationDraft, ProofObligationGroup, ProofObligationStatus, group_obligations,
    total_counts,
};
use super::{ChangeProofContractDelta, ContractChangeKind, ContractFamily, ProofObligations};
use crate::review::model::ReviewReport;
use crate::review::signals::tiered::SignalFamily;
use crate::verification::{VerificationRole, VerificationStatus};
use std::collections::{BTreeMap, BTreeSet};

pub(super) fn derive_verification_obligations(
    report: &ReviewReport,
    contract_deltas: &[ChangeProofContractDelta],
) -> (ProofObligations, bool, Vec<ProofObligationGroup>) {
    let groups = group_obligations(obligation_drafts(report, contract_deltas));
    let obligations = total_counts(&groups);
    let sufficient_policy = obligations.applicable > 0
        && obligations.unavailable == 0
        && obligations.unselected == 0
        && obligations.stale == 0;
    (obligations, sufficient_policy, groups)
}

fn obligation_drafts(
    report: &ReviewReport,
    contract_deltas: &[ChangeProofContractDelta],
) -> Vec<ProofObligationDraft> {
    let mut required = required_verification_requirements(report);
    required.extend(contract_deltas.iter().filter_map(contract_requirement));
    let selected = report
        .verification_policy
        .selected
        .iter()
        .cloned()
        .collect::<BTreeSet<_>>();
    let mut consumed = BTreeSet::new();
    let mut drafts = required
        .into_iter()
        .map(|requirement| draft_required_obligation(report, &selected, &mut consumed, requirement))
        .collect::<Vec<_>>();
    add_unmatched_outcome_drafts(report, &consumed, &mut drafts);
    add_orphan_selected_drafts(report, &selected, &consumed, &mut drafts);
    drafts
}

fn draft_required_obligation(
    report: &ReviewReport,
    selected: &BTreeSet<String>,
    consumed: &mut BTreeSet<String>,
    requirement: VerificationRequirement,
) -> ProofObligationDraft {
    let configured = report
        .verification_policy
        .configured
        .iter()
        .filter(|check| check.role == requirement.role && check.matches_path(&requirement.path))
        .collect::<Vec<_>>();
    if configured.is_empty() {
        return required_draft(requirement, Vec::new(), ProofObligationStatus::Unavailable);
    }
    let selected_for_role = configured
        .iter()
        .filter(|check| selected.contains(&check.id))
        .map(|check| check.id.as_str())
        .collect::<Vec<_>>();
    if selected_for_role.is_empty() {
        return required_draft(
            requirement,
            configured.iter().map(|check| check.id.clone()).collect(),
            ProofObligationStatus::Unselected,
        );
    }
    consumed.extend(selected_for_role.iter().map(|id| (*id).to_string()));
    let status = required_check_status(
        &report.verification,
        &selected_for_role,
        requirement.role,
        report.analysis_revision.as_deref(),
    );
    required_draft(
        requirement,
        selected_for_role.into_iter().map(str::to_string).collect(),
        status,
    )
}

fn required_draft(
    requirement: VerificationRequirement,
    check_ids: Vec<String>,
    status: ProofObligationStatus,
) -> ProofObligationDraft {
    ProofObligationDraft {
        role: Some(requirement.role),
        path: Some(requirement.path),
        check_ids,
        status,
    }
}

fn add_unmatched_outcome_drafts(
    report: &ReviewReport,
    consumed: &BTreeSet<String>,
    drafts: &mut Vec<ProofObligationDraft>,
) {
    let mut by_id = BTreeMap::new();
    for outcome in &report.verification {
        if !consumed.contains(&outcome.check_id) {
            by_id
                .entry(outcome.check_id.as_str())
                .or_insert_with(Vec::new)
                .push(outcome);
        }
    }
    for (id, outcomes) in by_id {
        let outcome = outcomes[0];
        let configured = report
            .verification_policy
            .configured
            .iter()
            .find(|check| check.id == id);
        let status = if outcomes.len() != 1
            || configured.is_some_and(|check| check.role != outcome.role)
            || (!report.verification_policy.configured.is_empty()
                && (configured.is_none()
                    || !report.verification_policy.selected.iter().any(|check| check == id)))
        {
            ProofObligationStatus::Unavailable
        } else {
            outcome_state(outcome, report.analysis_revision.as_deref())
        };
        drafts.push(ProofObligationDraft {
            role: Some(configured.map_or(outcome.role, |check| check.role)),
            path: None,
            check_ids: vec![id.to_string()],
            status,
        });
    }
}

fn add_orphan_selected_drafts(
    report: &ReviewReport,
    selected: &BTreeSet<String>,
    consumed: &BTreeSet<String>,
    drafts: &mut Vec<ProofObligationDraft>,
) {
    drafts.extend(selected.iter().filter_map(|check_id| {
        let has_outcome = report
            .verification
            .iter()
            .any(|outcome| &outcome.check_id == check_id);
        if has_outcome || consumed.contains(check_id) {
            return None;
        }
        Some(ProofObligationDraft {
            role: report
                .verification_policy
                .configured
                .iter()
                .find(|check| check.id == *check_id)
                .map(|check| check.role),
            path: None,
            check_ids: vec![check_id.clone()],
            status: ProofObligationStatus::Unavailable,
        })
    }));
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
struct VerificationRequirement {
    role: VerificationRole,
    path: String,
}

fn required_verification_requirements(report: &ReviewReport) -> BTreeSet<VerificationRequirement> {
    report
        .tiered_signals
        .definitely
        .iter()
        // Re-running a suite cannot confirm a test the change stopped running,
        // so integrity signals never become a check obligation; they keep the
        // proof at REVIEW until resolved or acknowledged.
        .filter(|signal| {
            !signal.suppressed
                && signal.verification_plan.is_some()
                && signal.family != SignalFamily::Integrity
        })
        .map(|signal| VerificationRequirement {
            role: match (signal.family, signal.kind.as_str()) {
                (
                    SignalFamily::Behavioral,
                    "behavioral.removed-export-still-imported"
                    | "behavioral.rust-public-function-arity-changed"
                    | "behavioral.rust-public-struct-field-removed"
                    | "behavioral.rust-public-enum-variant-removed",
                ) => VerificationRole::TypeCheck,
                _ => VerificationRole::Test,
            },
            path: signal.path.clone(),
        })
        .collect()
}

fn contract_requirement(delta: &ChangeProofContractDelta) -> Option<VerificationRequirement> {
    if delta.change == ContractChangeKind::MetadataOnly {
        return None;
    }
    let role = match (delta.family, delta.change) {
        (
            ContractFamily::PublicSymbol,
            ContractChangeKind::RemovedExport
            | ContractChangeKind::FunctionArityChanged
            | ContractChangeKind::StructFieldRemoved,
        ) => VerificationRole::TypeCheck,
        (ContractFamily::Delivery, _)
        | (ContractFamily::TestCoverage, ContractChangeKind::TestChanged)
        | (ContractFamily::SecurityBoundary, ContractChangeKind::EntryPointImpacted) => {
            return None;
        }
        (ContractFamily::Dependency, _) => VerificationRole::Build,
        (ContractFamily::RuntimeConfiguration, _) => VerificationRole::Test,
        (ContractFamily::SecurityBoundary, ContractChangeKind::BoundaryChanged)
        | (ContractFamily::TestCoverage, ContractChangeKind::TestMissing) => VerificationRole::Test,
        _ => return None,
    };
    let path = if delta.family == ContractFamily::Dependency {
        &delta.exporter_path
    } else {
        &delta.consumer_path
    };
    Some(VerificationRequirement {
        role,
        path: path.clone(),
    })
}

fn required_check_status(
    outcomes: &[crate::verification::VerificationOutcome],
    selected: &[&str],
    role: VerificationRole,
    analysis_revision: Option<&str>,
) -> ProofObligationStatus {
    let states = selected
        .iter()
        .map(|id| {
            let mut matching = outcomes.iter().filter(|outcome| outcome.check_id == *id);
            let Some(outcome) = matching.next() else {
                return ProofObligationStatus::Unavailable;
            };
            if matching.next().is_some() || outcome.role != role {
                return ProofObligationStatus::Unavailable;
            }
            outcome_state(outcome, analysis_revision)
        })
        .collect::<Vec<_>>();
    aggregate_states(&states)
}

fn outcome_state(
    outcome: &crate::verification::VerificationOutcome,
    analysis_revision: Option<&str>,
) -> ProofObligationStatus {
    let revision_matches = outcome.matches_revision(analysis_revision);
    match outcome.status {
        VerificationStatus::Passed if outcome.is_valid_pass(analysis_revision) => {
            ProofObligationStatus::Satisfied
        }
        VerificationStatus::Passed if revision_matches => ProofObligationStatus::Unavailable,
        VerificationStatus::Passed => ProofObligationStatus::Stale,
        VerificationStatus::Failed => ProofObligationStatus::Failed,
        VerificationStatus::TimedOut
        | VerificationStatus::Unavailable
        | VerificationStatus::Cancelled => ProofObligationStatus::Unavailable,
        VerificationStatus::Skipped if revision_matches => ProofObligationStatus::Unavailable,
        VerificationStatus::Skipped => ProofObligationStatus::Stale,
    }
}

fn aggregate_states(states: &[ProofObligationStatus]) -> ProofObligationStatus {
    if states
        .iter()
        .any(|state| matches!(state, ProofObligationStatus::Failed))
    {
        ProofObligationStatus::Failed
    } else if states
        .iter()
        .any(|state| matches!(state, ProofObligationStatus::Unavailable))
    {
        ProofObligationStatus::Unavailable
    } else if states
        .iter()
        .any(|state| matches!(state, ProofObligationStatus::Stale))
    {
        ProofObligationStatus::Stale
    } else if states
        .iter()
        .any(|state| matches!(state, ProofObligationStatus::Satisfied))
    {
        ProofObligationStatus::Satisfied
    } else {
        ProofObligationStatus::Unselected
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::verification::VerificationOutcome;

    fn passed(id: &str) -> VerificationOutcome {
        VerificationOutcome {
            check_id: id.to_string(),
            role: VerificationRole::Test,
            status: VerificationStatus::Passed,
            duration_ms: 0,
            exit_code: Some(0),
            working_directory: ".".to_string(),
            stdout_excerpt: String::new(),
            stderr_excerpt: String::new(),
            stdout_truncated: false,
            stderr_truncated: false,
            revision_before: "current".to_string(),
            revision_after: "current".to_string(),
            revision_compatible: true,
            limitations: Vec::new(),
            reused: false,
            diagnostics: None,
        }
    }

    #[test]
    fn every_selected_required_check_must_return_one_matching_outcome() {
        let first = passed("unit");
        let second = passed("integration");
        let selected = ["unit", "integration"];
        let role = VerificationRole::Test;
        assert_eq!(
            required_check_status(
                std::slice::from_ref(&first),
                &selected,
                role,
                Some("current"),
            ),
            ProofObligationStatus::Unavailable
        );
        assert_eq!(
            required_check_status(
                &[first.clone(), second.clone()],
                &selected,
                role,
                Some("current"),
            ),
            ProofObligationStatus::Satisfied
        );
        assert_eq!(
            required_check_status(
                &[first.clone(), first, second],
                &selected,
                role,
                Some("current"),
            ),
            ProofObligationStatus::Unavailable
        );
    }

    #[test]
    fn wrong_role_or_revision_cannot_satisfy_requirement() {
        let mut outcome = passed("unit");
        assert_eq!(
            outcome_state(&outcome, Some("other")),
            ProofObligationStatus::Stale
        );
        outcome.role = VerificationRole::Lint;
        assert_eq!(
            required_check_status(
                &[outcome],
                &["unit"],
                VerificationRole::Test,
                Some("current"),
            ),
            ProofObligationStatus::Unavailable
        );
    }
}
