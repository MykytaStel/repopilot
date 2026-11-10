mod finish;
mod plan;

use crate::cli::ReviewOptions;
use repopilot::config::model::ReviewScope;
use repopilot::findings::filter::FindingFilter;
use repopilot::findings::visibility::FindingVisibilityProfile;
use repopilot::review::ReviewInput;
use repopilot::review::ReviewSignalGatePolicy;
use repopilot::review::diff::OwnedDiffTarget;
use repopilot::review::intent::IntentContext;
use std::time::Duration;

pub fn run(options: ReviewOptions) -> Result<(), Box<dyn std::error::Error>> {
    let plan = plan::prepare(&options)?;
    let scan = plan::scan(&options, &plan)?;
    let mut report = plan::build_report(&options, &plan, scan.summary, &scan.session)?;
    if plan::explain_signal(&options, &report, plan.review_gate_policy)? {
        return Ok(());
    }
    finish::complete(&options, &plan, &scan.session, &mut report)
}

pub(super) struct ReviewPlan {
    pub(super) priority_filter: FindingFilter,
    pub(super) scope: ReviewScope,
    pub(super) visibility_profile: FindingVisibilityProfile,
    pub(super) review_gate_policy: ReviewSignalGatePolicy,
    pub(super) review_input: ReviewInput,
    pub(super) history_target: OwnedDiffTarget,
    pub(super) intent: IntentContext,
    pub(super) diff_loading_us: u64,
}

pub(super) fn duration_us(duration: Duration) -> u64 {
    duration.as_micros().min(u128::from(u64::MAX)) as u64
}
