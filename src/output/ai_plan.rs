use crate::findings::types::{Finding, FindingCategory, Severity};
use crate::graph::context::{ContextGraphFileMetric, ContextGraphSummary};
use crate::output::ai_context::{AiFocusCategory, DEFAULT_TOKEN_BUDGET, project_name};
use crate::output::finding_helpers::{
    RuleCluster, category_rank, clusters_by_rule_scope, example_locations, finding_recommendation,
};
use crate::scan::types::ScanSummary;
use std::fmt::Write as FmtWrite;

pub struct AiPlanOptions {
    pub focus: Option<AiFocusCategory>,
    pub budget_tokens: usize,
}

impl Default for AiPlanOptions {
    fn default() -> Self {
        Self {
            focus: None,
            budget_tokens: DEFAULT_TOKEN_BUDGET,
        }
    }
}

pub fn render(summary: &ScanSummary, opts: &AiPlanOptions) -> String {
    let project_name = project_name(summary);
    let budget_chars = opts.budget_tokens.saturating_mul(4);

    let findings: Vec<&Finding> = summary
        .artifacts
        .findings
        .iter()
        .filter(|finding| {
            opts.focus
                .as_ref()
                .is_none_or(|focus| focus.matches(&finding.category))
        })
        .collect();
    let mut clusters = clusters_by_rule_scope(&findings);
    sort_ai_plan_clusters(&mut clusters);

    let mut out = String::new();
    let _ = writeln!(out, "# RepoPilot AI Plan - {project_name}\n");
    let _ = writeln!(
        out,
        "Prioritized remediation plan generated locally from RepoPilot findings. Start at P0 and stop when the remaining risk is acceptable for this release.\n"
    );
    render_summary(&mut out, &findings);
    render_context_graph_plan(&mut out, summary.artifacts.context_graph_summary.as_ref());

    if findings.is_empty() {
        let _ = writeln!(out, "No findings matched the selected scope.");
        render_footer(&mut out, summary.scan_duration_us);
        return out;
    }

    let mut current_priority = None;
    let content_start = out.len();

    for (index, cluster) in clusters.iter().enumerate() {
        let priority = priority_label(cluster_priority(cluster));
        if current_priority != Some(priority) {
            let _ = writeln!(out, "\n## {priority}");
            current_priority = Some(priority);
        }

        let len_before = out.len();
        render_cluster_plan(&mut out, cluster, index + 1);
        let content_used = out.len().saturating_sub(content_start);
        if content_used > budget_chars {
            if index == 0 {
                let _ = writeln!(
                    out,
                    "\n*[Single cluster exceeds token budget — output may be long]*"
                );
            } else {
                out.truncate(len_before);
                let _ = writeln!(out, "\n*[Plan truncated to stay within token budget]*");
            }
            break;
        }
    }

    render_verification(&mut out);
    render_footer(&mut out, summary.scan_duration_us);
    out
}

include!("ai_plan/summary.rs");
include!("ai_plan/cluster.rs");
include!("ai_plan/priority.rs");
