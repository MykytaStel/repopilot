use crate::baseline::diff::{BaselineStatus, FindingBaselineStatus};
use crate::findings::types::{Finding, Severity};
use std::collections::{HashMap, HashSet};
use std::path::Path;

use super::model::{
    FORMULA_VERSION, RiskInputs, RiskSignal, clamp_score, priority_for_score, signal,
};

pub fn apply_baseline_overlay(
    findings: &mut [Finding],
    statuses: &[FindingBaselineStatus],
    root: &Path,
) {
    let status_by_key = statuses
        .iter()
        .map(|status| (status.key.as_str(), status.status))
        .collect::<HashMap<_, _>>();

    for finding in findings {
        let key = crate::baseline::key::stable_finding_key(finding, root);
        if let Some(status) = status_by_key.get(key.as_str()).copied() {
            apply_overlay_signal(
                finding,
                baseline_signal(status),
                RiskInputs {
                    baseline_status: Some(status),
                    ..RiskInputs::default()
                },
            );
        }
    }
}

pub fn apply_review_overlay(findings: &mut [Finding], in_diff: &[bool]) {
    for (finding, in_diff) in findings.iter_mut().zip(in_diff.iter().copied()) {
        if in_diff {
            apply_overlay_signal(
                finding,
                signal(
                    "review.in-diff",
                    "changed lines",
                    12,
                    "finding touches changed diff lines",
                ),
                RiskInputs {
                    in_diff: true,
                    ..RiskInputs::default()
                },
            );
        }
    }
}

pub fn apply_workspace_hotspot_overlay(findings: &mut [Finding]) {
    let hotspot_packages = workspace_hotspots(findings);
    if hotspot_packages.is_empty() {
        return;
    }

    for finding in findings {
        if finding
            .workspace_package
            .as_ref()
            .is_some_and(|package| hotspot_packages.contains(package))
        {
            apply_overlay_signal(
                finding,
                signal(
                    "workspace.hotspot",
                    "workspace hotspot",
                    5,
                    "workspace package has multiple high-risk findings",
                ),
                RiskInputs {
                    workspace_hotspot: true,
                    ..RiskInputs::default()
                },
            );
        }
    }
}

pub(super) fn baseline_signal(status: BaselineStatus) -> RiskSignal {
    match status {
        BaselineStatus::New => signal(
            "baseline.new",
            "new finding",
            10,
            "new findings should be prioritized over accepted existing debt",
        ),
        BaselineStatus::Existing => signal(
            "baseline.existing",
            "existing finding",
            -8,
            "existing baseline findings are already accepted technical debt",
        ),
    }
}

fn apply_overlay_signal(finding: &mut Finding, signal: RiskSignal, _inputs: RiskInputs) {
    if finding
        .risk
        .signals
        .iter()
        .any(|existing| existing.id == signal.id)
    {
        return;
    }

    let score = finding.risk.score as i16 + signal.weight;
    finding.risk.signals.push(signal);
    finding.risk.score = clamp_score(score);
    finding.risk.priority = priority_for_score(finding.risk.score);
    finding.risk.formula_version = FORMULA_VERSION.to_string();
}

fn workspace_hotspots(findings: &[Finding]) -> HashSet<String> {
    let mut counts: HashMap<String, usize> = HashMap::new();
    for finding in findings {
        if finding.severity >= Severity::High
            && let Some(package) = &finding.workspace_package
        {
            *counts.entry(package.clone()).or_default() += 1;
        }
    }
    counts
        .into_iter()
        .filter_map(|(package, count)| (count > 1).then_some(package))
        .collect()
}
