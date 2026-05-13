use crate::findings::types::{Finding, Severity};
use crate::output::finding_helpers::{
    RuleCluster, clusters_by_rule, example_locations, finding_recommendation,
};
use std::fmt::Write as FmtWrite;

pub(super) fn render_top_recommendations(out: &mut String, findings: &[&Finding]) {
    let mut top = recommendation_clusters(findings, Severity::High);
    if top.is_empty() {
        top = recommendation_clusters(findings, Severity::Medium);
    }
    top.truncate(5);

    if top.is_empty() {
        return;
    }

    let _ = writeln!(out, "## Top Recommendations");
    out.push('\n');

    for (i, cluster) in top.iter().enumerate() {
        let recommendation = cluster
            .findings
            .first()
            .map(|finding| finding_recommendation(finding))
            .unwrap_or_default();
        let examples = example_locations(&cluster.findings, 3);
        let examples = if examples.is_empty() {
            String::new()
        } else {
            format!(" Examples: {}.", examples.join(", "))
        };
        let _ = writeln!(
            out,
            "{}. **{}** - {} {} finding(s).{} {}",
            i + 1,
            cluster.title,
            cluster.severity.label(),
            cluster.findings.len(),
            examples,
            recommendation
        );
    }
    out.push('\n');
}

fn recommendation_clusters<'a>(
    findings: &'a [&'a Finding],
    min_severity: Severity,
) -> Vec<RuleCluster<'a>> {
    let mut clusters = clusters_by_rule(findings)
        .into_iter()
        .filter(|cluster| cluster.severity >= min_severity && !cluster.findings.is_empty())
        .collect::<Vec<_>>();

    clusters.sort_by(|left, right| {
        right
            .severity
            .cmp(&left.severity)
            .then_with(|| right.findings.len().cmp(&left.findings.len()))
            .then_with(|| left.title.cmp(right.title))
    });
    clusters
}

pub(super) fn render_footer(
    out: &mut String,
    content_len: usize,
    budget_tokens: usize,
    scan_duration_us: u64,
) {
    let approx_tokens = content_len / 4;
    let scan_ms = scan_duration_us / 1000;
    let _ = writeln!(
        out,
        "---\n*~{approx_tokens} tokens (budget: {budget_tokens}) · scanned in {scan_ms}ms — paste into Claude Code, Cursor, or ChatGPT to start fixing*"
    );
}
