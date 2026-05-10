use crate::findings::types::{Finding, Severity};
use crate::rules::lookup_rule_metadata;
use std::collections::BTreeMap;
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
            cluster.recommendation
        );
    }
    out.push('\n');
}

struct RecommendationCluster<'a> {
    title: &'a str,
    severity: Severity,
    findings: Vec<&'a Finding>,
    recommendation: &'a str,
}

fn recommendation_clusters<'a>(
    findings: &'a [&'a Finding],
    min_severity: Severity,
) -> Vec<RecommendationCluster<'a>> {
    let mut by_rule: BTreeMap<&str, Vec<&Finding>> = BTreeMap::new();
    for finding in findings.iter().copied() {
        if finding.severity >= min_severity && finding_recommendation(finding).is_some() {
            by_rule.entry(&finding.rule_id).or_default().push(finding);
        }
    }

    let mut clusters = by_rule
        .into_values()
        .filter_map(|mut group| {
            group.sort_by(|left, right| {
                right
                    .severity
                    .cmp(&left.severity)
                    .then_with(|| finding_location(left).cmp(&finding_location(right)))
            });
            let first = group.first().copied()?;
            let title = cluster_title(first, group.len());
            Some(RecommendationCluster {
                title,
                severity: group
                    .iter()
                    .map(|finding| finding.severity)
                    .max()
                    .unwrap_or(first.severity),
                recommendation: finding_recommendation(first)?,
                findings: group,
            })
        })
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

fn cluster_title(finding: &Finding, count: usize) -> &str {
    if count > 1 {
        lookup_rule_metadata(&finding.rule_id)
            .map(|metadata| metadata.title)
            .unwrap_or(finding.rule_id.as_str())
    } else {
        finding.title.as_str()
    }
}

fn finding_recommendation(finding: &Finding) -> Option<&str> {
    lookup_rule_metadata(&finding.rule_id)
        .and_then(|meta| meta.recommendation)
        .or(if finding.description.is_empty() {
            None
        } else {
            Some(finding.description.as_str())
        })
}

fn example_locations(findings: &[&Finding], limit: usize) -> Vec<String> {
    findings
        .iter()
        .filter_map(|finding| finding_location(finding))
        .take(limit)
        .map(|location| format!("`{location}`"))
        .collect()
}

fn finding_location(finding: &Finding) -> Option<String> {
    finding.evidence.first().map(|evidence| {
        let path = evidence.path.display().to_string();
        if evidence.line_start > 0 {
            format!("{path}:{}", evidence.line_start)
        } else {
            path
        }
    })
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
