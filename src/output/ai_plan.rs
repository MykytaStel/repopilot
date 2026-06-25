//! Prioritized remediation plan sections, embedded by the unified `ai context`
//! handoff (`super::ai_context`). The plan groups findings into P0–P3 clusters
//! and surfaces the Context Risk Graph edit order; it is no longer a standalone
//! command.

use crate::findings::types::{Finding, FindingCategory, Severity};
use crate::output::ai_context::AiFocusCategory;
use crate::output::finding_helpers::{
    RuleCluster, category_rank, clusters_by_rule_scope, example_locations, finding_location,
    finding_recommendation,
};
use crate::scan::types::ScanSummary;
use serde::Serialize;
use std::fmt::Write as FmtWrite;

/// Renders the prioritized P0–P3 remediation clusters for embedding in the
/// `ai context` handoff — no document title or footer. The Context Risk Graph
/// edit order is rendered separately by `super::ai_context`'s hot-files section,
/// so it is not repeated here. Emits nothing when no finding matches `focus`.
pub(crate) fn render_plan_section(
    out: &mut String,
    summary: &ScanSummary,
    focus: Option<&AiFocusCategory>,
    budget_chars: usize,
) {
    let findings: Vec<&Finding> = summary
        .artifacts
        .findings
        .iter()
        .filter(|finding| focus.is_none_or(|focus| focus.matches(&finding.category)))
        .collect();
    if findings.is_empty() {
        return;
    }
    let mut clusters = clusters_by_rule_scope(&findings);
    sort_ai_plan_clusters(&mut clusters);

    let _ = writeln!(
        out,
        "\n## Remediation Plan\n\nPrioritized from RepoPilot findings — start at P0 and stop when the remaining risk is acceptable for this release."
    );

    let mut current_priority = None;
    let content_start = out.len();
    for (index, cluster) in clusters.iter().enumerate() {
        let priority = priority_label(cluster_priority(cluster));
        if current_priority != Some(priority) {
            let _ = writeln!(out, "\n### {priority}");
            current_priority = Some(priority);
        }

        let len_before = out.len();
        render_cluster_plan(out, cluster, index + 1);
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
}

/// Serializable view of one prioritized plan cluster, for `ai context --format
/// json`. Mirrors what `render_cluster_plan` emits as Markdown.
#[derive(Serialize)]
pub(crate) struct PlanCluster {
    pub(crate) priority: &'static str,
    pub(crate) title: String,
    pub(crate) rule_id: String,
    pub(crate) severity: &'static str,
    pub(crate) max_score: u8,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) scope: Option<String>,
    pub(crate) finding_count: usize,
    pub(crate) examples: Vec<String>,
    pub(crate) recommendation: String,
}

/// The ordered P0–P3 plan clusters as data — same ordering and priority labels
/// as the Markdown `render_plan_section`, but structured for the JSON handoff.
pub(crate) fn ordered_plan_clusters(
    summary: &ScanSummary,
    focus: Option<&AiFocusCategory>,
) -> Vec<PlanCluster> {
    let findings: Vec<&Finding> = summary
        .artifacts
        .findings
        .iter()
        .filter(|finding| focus.is_none_or(|focus| focus.matches(&finding.category)))
        .collect();
    let mut clusters = clusters_by_rule_scope(&findings);
    sort_ai_plan_clusters(&mut clusters);

    clusters
        .iter()
        .map(|cluster| {
            let first = cluster.findings[0];
            PlanCluster {
                priority: priority_label(cluster_priority(cluster)),
                title: cluster.title.clone(),
                rule_id: cluster.rule_id.to_string(),
                severity: cluster.severity.label(),
                max_score: cluster.max_score,
                scope: cluster.scope.clone().filter(|scope| scope != "."),
                finding_count: cluster.findings.len(),
                examples: cluster
                    .findings
                    .iter()
                    .filter_map(|finding| finding_location(finding))
                    .take(3)
                    .collect(),
                recommendation: finding_recommendation(first).to_string(),
            }
        })
        .collect()
}

include!("ai_plan/cluster.rs");
include!("ai_plan/priority.rs");
