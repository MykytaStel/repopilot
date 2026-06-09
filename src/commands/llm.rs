use crate::commands::focus::parse_focus_category;
use crate::commands::product_scan::{
    ProductScanMode, ProductScanRequest, emit_report_only_diagnostics,
    enforce_diagnostics_exit_policy, run_product_scan_with_facts_summary,
};
use crate::commands::scan_config::ScanConfigOverrides;
use repopilot::facts::RepoFactsSummary;
use repopilot::findings::filter::FindingFilter;
use repopilot::findings::visibility::FindingVisibilityProfile;
use repopilot::output::ai_context::{AiFocusCategory, DEFAULT_TOKEN_BUDGET};
use repopilot::report::writer::write_report;
use repopilot::scan::types::ScanSummary;
use std::path::PathBuf;

pub struct LlmCommandArgs {
    pub path: PathBuf,
    pub config: Option<PathBuf>,
    pub focus: Option<String>,
    pub budget: Option<usize>,
    pub output: Option<PathBuf>,
}

pub fn run_markdown_command<F>(
    args: LlmCommandArgs,
    render: F,
) -> Result<(), Box<dyn std::error::Error>>
where
    F: FnOnce(&ScanSummary, Option<&RepoFactsSummary>, Option<AiFocusCategory>, usize) -> String,
{
    let focus_category = parse_focus_category(args.focus.as_deref())?;
    let budget_tokens = args.budget.unwrap_or(DEFAULT_TOKEN_BUDGET);

    let scan_result = run_product_scan_with_facts_summary(ProductScanRequest {
        path: args.path,
        config_path: args.config,
        overrides: ScanConfigOverrides::default(),
        preset: None,
        mode: ProductScanMode::Full,
        no_progress: false,
        ignore_feedback: false,
        visibility_profile: FindingVisibilityProfile::Default,
        pre_visibility_filter: FindingFilter::default(),
    })?;
    let summary = scan_result.summary;

    emit_report_only_diagnostics(&summary);
    let rendered = render(
        &summary,
        scan_result.repo_facts_summary.as_ref(),
        focus_category,
        budget_tokens,
    );
    write_report(&rendered, args.output.as_deref())?;
    enforce_diagnostics_exit_policy(&summary)?;

    Ok(())
}
