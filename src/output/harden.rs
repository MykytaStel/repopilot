use crate::findings::types::{Finding, FindingCategory, Severity};
use crate::output::finding_helpers::{
    RuleCluster, category_rank, clusters_by_rule_scope, example_locations, finding_recommendation,
};
use crate::output::vibe::{DEFAULT_TOKEN_BUDGET, VibeCategory, project_name};
use crate::scan::types::ScanSummary;
use std::fmt::Write as FmtWrite;

pub struct HardenOptions {
    pub focus: Option<VibeCategory>,
    pub budget_tokens: usize,
}

impl Default for HardenOptions {
    fn default() -> Self {
        Self {
            focus: None,
            budget_tokens: DEFAULT_TOKEN_BUDGET,
        }
    }
}

pub fn render(summary: &ScanSummary, opts: &HardenOptions) -> String {
    let project_name = project_name(summary);
    let budget_chars = opts.budget_tokens.saturating_mul(4);

    let findings: Vec<&Finding> = summary
        .findings
        .iter()
        .filter(|finding| {
            opts.focus
                .as_ref()
                .is_none_or(|focus| focus.matches(&finding.category))
        })
        .collect();
    let mut clusters = clusters_by_rule_scope(&findings);
    sort_harden_clusters(&mut clusters);

    let mut out = String::new();
    let _ = writeln!(out, "# RepoPilot Harden Plan - {project_name}\n");
    let _ = writeln!(
        out,
        "Prioritized remediation plan generated locally from RepoPilot findings. Start at P0 and stop when the remaining risk is acceptable for this release.\n"
    );
    render_summary(&mut out, &findings);

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

fn render_summary(out: &mut String, findings: &[&Finding]) {
    let critical = findings
        .iter()
        .filter(|finding| finding.severity == Severity::Critical)
        .count();
    let high = findings
        .iter()
        .filter(|finding| finding.severity == Severity::High)
        .count();
    let medium = findings
        .iter()
        .filter(|finding| finding.severity == Severity::Medium)
        .count();
    let low = findings
        .iter()
        .filter(|finding| finding.severity == Severity::Low)
        .count();
    let _ = writeln!(
        out,
        "## Priority Summary\n\n- Total: {} findings\n- Critical: {critical}\n- High: {high}\n- Medium: {medium}\n- Low: {low}",
        findings.len()
    );
}

fn render_cluster_plan(out: &mut String, cluster: &RuleCluster<'_>, index: usize) {
    let count_note = if cluster.findings.len() > 1 {
        format!(" ({} findings)", cluster.findings.len())
    } else {
        String::new()
    };
    let _ = writeln!(
        out,
        "\n### {index}. [{}] {}{}",
        cluster.severity.label(),
        cluster.title,
        count_note
    );
    let _ = writeln!(out, "- Rule: `{}`", cluster.rule_id);
    if let Some(scope) = &cluster.scope
        && scope != "."
    {
        let _ = writeln!(out, "- Area: `{scope}`");
    }
    let _ = writeln!(
        out,
        "- Priority: {} (max risk {}/100)",
        cluster.priority.label(),
        cluster.max_score
    );

    let examples = example_locations(&cluster.findings, 3);
    if !examples.is_empty() {
        let _ = writeln!(out, "- Examples: {}", examples.join(", "));
    }

    let first = cluster.findings[0];
    if !first.description.is_empty() {
        let _ = writeln!(out, "- Why: {}", first.description);
    }

    let _ = writeln!(out, "- Fix: {}", finding_recommendation(first));
}

fn render_verification(out: &mut String) {
    let _ = writeln!(
        out,
        "\n## Verify\n\n- Run `repopilot scan . --min-severity high` after P0/P1 fixes.\n- Run `repopilot review . --base origin/main --fail-on new-high` before merging.\n- Refresh a baseline only when the remaining findings are accepted technical debt."
    );
}

fn render_footer(out: &mut String, scan_duration_us: u64) {
    let scan_ms = scan_duration_us / 1000;
    let _ = writeln!(out, "\n---\n*Generated by RepoPilot in {scan_ms}ms.*");
}

fn priority_label(priority: u8) -> &'static str {
    match priority {
        0 => "P0 - Immediate risk",
        1 => "P1 - High-impact hardening",
        2 => "P2 - Quality and maintainability",
        _ => "P3 - Backlog cleanup",
    }
}

fn sort_harden_clusters(clusters: &mut [RuleCluster<'_>]) {
    clusters.sort_by(|left, right| {
        priority_rank(left)
            .cmp(&priority_rank(right))
            .then_with(|| right.max_score.cmp(&left.max_score))
            .then_with(|| cluster_priority(left).cmp(&cluster_priority(right)))
            .then_with(|| right.severity.cmp(&left.severity))
            .then_with(|| cluster_category_rank(left).cmp(&cluster_category_rank(right)))
            .then_with(|| right.findings.len().cmp(&left.findings.len()))
            .then_with(|| left.rule_id.cmp(right.rule_id))
    });
}

fn cluster_priority(cluster: &RuleCluster<'_>) -> u8 {
    cluster
        .findings
        .iter()
        .map(|finding| legacy_priority_rank(finding))
        .min()
        .unwrap_or(3)
}

fn priority_rank(cluster: &RuleCluster<'_>) -> u8 {
    match cluster.priority {
        crate::risk::RiskPriority::P0 => 0,
        crate::risk::RiskPriority::P1 => 1,
        crate::risk::RiskPriority::P2 => 2,
        crate::risk::RiskPriority::P3 => 3,
    }
}

fn legacy_priority_rank(finding: &Finding) -> u8 {
    if finding.severity == Severity::Critical
        || (finding.severity == Severity::High && finding.category == FindingCategory::Security)
    {
        0
    } else if finding.severity == Severity::High {
        1
    } else if finding.severity == Severity::Medium {
        2
    } else {
        3
    }
}

fn cluster_category_rank(cluster: &RuleCluster<'_>) -> u8 {
    cluster
        .findings
        .first()
        .map(|finding| category_rank(&finding.category))
        .unwrap_or(u8::MAX)
}
