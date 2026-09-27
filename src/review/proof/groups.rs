use super::ProofObligations;
use crate::verification::VerificationRole;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum ProofObligationStatus {
    Satisfied,
    Failed,
    Unavailable,
    Unselected,
    Stale,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ProofObligation {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub path: Option<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub check_ids: Vec<String>,
    pub status: ProofObligationStatus,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ProofObligationGroup {
    pub role: Option<VerificationRole>,
    pub counts: ProofObligations,
    pub obligations: Vec<ProofObligation>,
    pub next_action: String,
}

#[derive(Debug, Clone)]
pub(super) struct ProofObligationDraft {
    pub role: Option<VerificationRole>,
    pub path: Option<String>,
    pub check_ids: Vec<String>,
    pub status: ProofObligationStatus,
}

pub(super) fn group_obligations(drafts: Vec<ProofObligationDraft>) -> Vec<ProofObligationGroup> {
    let mut by_role = BTreeMap::<Option<VerificationRole>, Vec<ProofObligationDraft>>::new();
    for draft in drafts {
        by_role.entry(draft.role).or_default().push(draft);
    }

    let mut groups = by_role
        .into_iter()
        .map(|(role, mut drafts)| {
            drafts.sort_by_key(|draft| (draft.path.clone(), draft.check_ids.clone(), draft.status));
            let obligations = drafts
                .into_iter()
                .map(|draft| ProofObligation {
                    path: draft.path,
                    check_ids: draft.check_ids,
                    status: draft.status,
                })
                .collect::<Vec<_>>();
            let counts = count_obligations(&obligations);
            ProofObligationGroup {
                role,
                next_action: group_next_action(role, counts),
                counts,
                obligations,
            }
        })
        .collect::<Vec<_>>();
    groups.sort_by_key(|group| group.role.map(role_label).unwrap_or("unspecified"));
    groups
}

fn count_obligations(obligations: &[ProofObligation]) -> ProofObligations {
    let mut counts = ProofObligations {
        applicable: obligations.len(),
        satisfied: 0,
        failed: 0,
        unavailable: 0,
        unselected: 0,
        stale: 0,
    };
    for obligation in obligations {
        match obligation.status {
            ProofObligationStatus::Satisfied => counts.satisfied += 1,
            ProofObligationStatus::Failed => counts.failed += 1,
            ProofObligationStatus::Unavailable => counts.unavailable += 1,
            ProofObligationStatus::Unselected => counts.unselected += 1,
            ProofObligationStatus::Stale => counts.stale += 1,
        }
    }
    counts
}

fn group_next_action(role: Option<VerificationRole>, counts: ProofObligations) -> String {
    let role = role.map_or("unspecified", role_label);
    if counts.failed > 0 {
        format!("Fix the failed {role} checks first, then rerun the review.")
    } else if counts.stale > 0 {
        format!("Rerun the {role} checks against the current revision.")
    } else if counts.unavailable > 0 {
        format!("Make the required {role} checks available, then run them.")
    } else if counts.unselected > 0 {
        format!("Select and run the required {role} checks.")
    } else {
        format!("No action needed; all {role} obligations are satisfied.")
    }
}

pub(super) fn role_label(role: VerificationRole) -> &'static str {
    match role {
        VerificationRole::Test => "test",
        VerificationRole::Build => "build",
        VerificationRole::TypeCheck => "type-check",
        VerificationRole::Lint => "lint",
    }
}

pub(super) fn total_counts(groups: &[ProofObligationGroup]) -> ProofObligations {
    let mut total = ProofObligations {
        applicable: 0,
        satisfied: 0,
        failed: 0,
        unavailable: 0,
        unselected: 0,
        stale: 0,
    };
    for group in groups {
        total.applicable += group.counts.applicable;
        total.satisfied += group.counts.satisfied;
        total.failed += group.counts.failed;
        total.unavailable += group.counts.unavailable;
        total.unselected += group.counts.unselected;
        total.stale += group.counts.stale;
    }
    total
}
