use super::{ChangeProofContractDelta, ContractChangeKind, ContractFamily, ProofObligations};
use crate::review::model::ReviewReport;
use crate::review::signals::tiered::SignalFamily;
use crate::verification::{VerificationRole, VerificationStatus};
use std::collections::BTreeSet;

pub(super) fn derive_verification_obligations(
    report: &ReviewReport,
    contract_deltas: &[ChangeProofContractDelta],
) -> (ProofObligations, bool) {
    let mut required = required_verification_requirements(report);
    required.extend(contract_deltas.iter().filter_map(contract_requirement));
    let selected = report
        .verification_policy
        .selected
        .iter()
        .cloned()
        .collect::<BTreeSet<_>>();
    let mut consumed = BTreeSet::new();
    let mut states = Vec::new();

    for requirement in required {
        let configured = report
            .verification_policy
            .configured
            .iter()
            .filter(|check| check.role == requirement.role && check.matches_path(&requirement.path))
            .collect::<Vec<_>>();
        if configured.is_empty() {
            states.push(ObligationState::Unavailable);
            continue;
        }
        let selected_for_role = configured
            .iter()
            .filter(|check| selected.contains(&check.id))
            .map(|check| check.id.as_str())
            .collect::<Vec<_>>();
        if selected_for_role.is_empty() {
            states.push(ObligationState::Unselected);
            continue;
        }
        consumed.extend(selected_for_role.iter().map(|id| (*id).to_string()));
        let outcomes = report
            .verification
            .iter()
            .filter(|outcome| selected_for_role.contains(&outcome.check_id.as_str()))
            .map(outcome_state)
            .collect::<Vec<_>>();
        states.push(if outcomes.is_empty() {
            ObligationState::Unavailable
        } else {
            aggregate_states(&outcomes)
        });
    }

    states.extend(
        report
            .verification
            .iter()
            .filter(|outcome| !consumed.contains(&outcome.check_id))
            .map(outcome_state),
    );
    states.extend(
        selected
            .iter()
            .filter(|check_id| {
                !report
                    .verification
                    .iter()
                    .any(|outcome| &outcome.check_id == *check_id)
                    && !consumed.contains(*check_id)
            })
            .map(|_| ObligationState::Unavailable),
    );

    let mut obligations = ProofObligations {
        applicable: states.len(),
        satisfied: 0,
        failed: 0,
        unavailable: 0,
        unselected: 0,
        stale: 0,
    };
    for state in states {
        match state {
            ObligationState::Satisfied => obligations.satisfied += 1,
            ObligationState::Failed => obligations.failed += 1,
            ObligationState::Unavailable => obligations.unavailable += 1,
            ObligationState::Unselected => obligations.unselected += 1,
            ObligationState::Stale => obligations.stale += 1,
        }
    }
    let sufficient_policy = obligations.applicable > 0
        && obligations.unavailable == 0
        && obligations.unselected == 0
        && obligations.stale == 0;
    (obligations, sufficient_policy)
}

#[derive(Debug, Clone, Copy)]
enum ObligationState {
    Satisfied,
    Failed,
    Unavailable,
    Unselected,
    Stale,
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
        .filter(|signal| !signal.suppressed && signal.verification_plan.is_some())
        .map(|signal| VerificationRequirement {
            role: match (signal.family, signal.kind.as_str()) {
                (SignalFamily::Behavioral, "behavioral.removed-export-still-imported") => {
                    VerificationRole::TypeCheck
                }
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
        (ContractFamily::PublicSymbol, ContractChangeKind::RemovedExport) => {
            VerificationRole::TypeCheck
        }
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

fn outcome_state(outcome: &crate::verification::VerificationOutcome) -> ObligationState {
    match outcome.status {
        VerificationStatus::Passed if outcome.revision_compatible => ObligationState::Satisfied,
        VerificationStatus::Passed => ObligationState::Stale,
        VerificationStatus::Failed => ObligationState::Failed,
        VerificationStatus::TimedOut
        | VerificationStatus::Unavailable
        | VerificationStatus::Cancelled => ObligationState::Unavailable,
        VerificationStatus::Skipped => ObligationState::Unselected,
    }
}

fn aggregate_states(states: &[ObligationState]) -> ObligationState {
    if states
        .iter()
        .any(|state| matches!(state, ObligationState::Failed))
    {
        ObligationState::Failed
    } else if states
        .iter()
        .any(|state| matches!(state, ObligationState::Unavailable))
    {
        ObligationState::Unavailable
    } else if states
        .iter()
        .any(|state| matches!(state, ObligationState::Stale))
    {
        ObligationState::Stale
    } else if states
        .iter()
        .any(|state| matches!(state, ObligationState::Satisfied))
    {
        ObligationState::Satisfied
    } else {
        ObligationState::Unselected
    }
}
