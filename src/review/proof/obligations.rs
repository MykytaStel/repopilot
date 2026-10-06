use super::groups::{
    ProofObligationDraft, ProofObligationGroup, ProofObligationStatus, group_obligations,
    total_counts,
};
use super::{ChangeProofContractDelta, ContractChangeKind, ContractFamily, ProofObligations};
use crate::review::model::ReviewReport;
use crate::review::signals::tiered::SignalFamily;
use crate::verification::{VerificationRole, VerificationStatus};
use std::collections::BTreeSet;

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
    let outcomes = report
        .verification
        .iter()
        .filter(|outcome| selected_for_role.contains(&outcome.check_id.as_str()))
        .map(outcome_state)
        .collect::<Vec<_>>();
    let status = if outcomes.is_empty() {
        ProofObligationStatus::Unavailable
    } else {
        aggregate_states(&outcomes)
    };
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
    drafts.extend(
        report
            .verification
            .iter()
            .filter(|outcome| !consumed.contains(&outcome.check_id))
            .map(|outcome| ProofObligationDraft {
                role: Some(outcome.role),
                path: None,
                check_ids: vec![outcome.check_id.clone()],
                status: outcome_state(outcome),
            }),
    );
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
                    | "behavioral.rust-public-function-arity-changed",
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
            ContractChangeKind::RemovedExport | ContractChangeKind::FunctionArityChanged,
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

fn outcome_state(outcome: &crate::verification::VerificationOutcome) -> ProofObligationStatus {
    match outcome.status {
        VerificationStatus::Passed if outcome.revision_compatible => {
            ProofObligationStatus::Satisfied
        }
        VerificationStatus::Passed => ProofObligationStatus::Stale,
        VerificationStatus::Failed => ProofObligationStatus::Failed,
        VerificationStatus::TimedOut
        | VerificationStatus::Unavailable
        | VerificationStatus::Cancelled => ProofObligationStatus::Unavailable,
        VerificationStatus::Skipped if outcome.revision_compatible => {
            ProofObligationStatus::Unavailable
        }
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
