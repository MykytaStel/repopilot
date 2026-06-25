//! JSON form of the `ai context` handoff. The default Markdown brief is for a
//! human skimming; agents that parse structured data get the same facts —
//! evidence, snippets, risk, recommendation — without Markdown parsing, closing
//! the asymmetry with the JSON-returning MCP tools.
//!
//! Like the Markdown renderer it is **budget-aware**: findings are ordered by
//! risk and added until the serialized size reaches `--budget`, and the document
//! reports exactly what was included vs omitted. `approx_tokens` is measured from
//! the real (pretty) output, so an agent can trust it. Deterministic (no
//! clock/rand/net), so it can be golden-tested.

use crate::facts::RepoFactsSummary;
use crate::findings::types::{Finding, Severity};
use crate::output::ai_context::{AiContextRenderOptions, AiFocusCategory, project_name};
use crate::output::ai_plan::{PlanCluster, ordered_plan_clusters};
use crate::output::finding_helpers::finding_location_key;
use crate::scan::types::ScanSummary;
use serde::Serialize;

/// Bumped when the JSON shape changes in a breaking way.
pub const AI_CONTEXT_JSON_SCHEMA_VERSION: u32 = 1;

/// Share of the budget findings may consume before plan clusters get a turn, so
/// a finding-heavy repo still leaves room for the prioritized plan.
const FINDINGS_BUDGET_SHARE: usize = 70;

#[derive(Serialize)]
struct AiContextJson {
    schema_version: u32,
    project: String,
    focus: &'static str,
    budget_tokens: usize,
    approx_tokens: usize,
    truncated: bool,
    findings_total: usize,
    findings_included: usize,
    findings_omitted: usize,
    plan_clusters_total: usize,
    plan_clusters_included: usize,
    risk: RiskSummaryJson,
    #[serde(skip_serializing_if = "Option::is_none")]
    facts: Option<FactsJson>,
    findings: Vec<FindingJson>,
    plan: Vec<PlanCluster>,
}

#[derive(Serialize)]
struct RiskSummaryJson {
    /// Machine token, e.g. `moderate`.
    level: String,
    /// Display label, e.g. `🟡 MODERATE`.
    label: String,
    total: usize,
    critical: usize,
    high: usize,
    medium: usize,
    low: usize,
    info: usize,
}

#[derive(Serialize)]
struct FactsJson {
    total_files: usize,
    files_with_language: usize,
    total_non_empty_lines: usize,
    languages: Vec<LanguageJson>,
    diagnostics_count: usize,
}

#[derive(Serialize)]
struct LanguageJson {
    language: String,
    files: usize,
    non_empty_lines: usize,
}

#[derive(Serialize)]
struct FindingJson {
    id: String,
    rule_id: String,
    category: &'static str,
    severity: &'static str,
    confidence: &'static str,
    title: String,
    #[serde(skip_serializing_if = "String::is_empty")]
    description: String,
    #[serde(skip_serializing_if = "String::is_empty")]
    recommendation: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    docs_url: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    workspace_package: Option<String>,
    risk: FindingRiskJson,
    evidence: Vec<EvidenceJson>,
}

#[derive(Serialize)]
struct FindingRiskJson {
    score: u8,
    priority: &'static str,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    signals: Vec<RiskSignalJson>,
}

#[derive(Serialize)]
struct RiskSignalJson {
    id: String,
    label: String,
    weight: i16,
    reason: String,
}

#[derive(Serialize)]
struct EvidenceJson {
    path: String,
    line_start: usize,
    #[serde(skip_serializing_if = "Option::is_none")]
    line_end: Option<usize>,
    #[serde(skip_serializing_if = "String::is_empty")]
    snippet: String,
}

/// Render the AI context handoff as pretty JSON. Deterministic and never panics:
/// serializing this owned struct cannot realistically fail, but a failure is
/// surfaced as an `{"error": ...}` document rather than a panic.
pub fn render_json(
    summary: &ScanSummary,
    facts_summary: Option<&RepoFactsSummary>,
    opts: &AiContextRenderOptions,
) -> String {
    let mut doc = build(summary, facts_summary, opts);
    // Two-pass on the *pretty* encoding — the form the caller receives — so
    // `approx_tokens` reflects the real output, not a denser compact form.
    if let Ok(pretty) = serde_json::to_string_pretty(&doc) {
        doc.approx_tokens = pretty.len() / 4;
    }
    serde_json::to_string_pretty(&doc).unwrap_or_else(|error| {
        format!("{{\n  \"error\": \"ai context json serialization failed: {error}\"\n}}")
    })
}

fn build(
    summary: &ScanSummary,
    facts_summary: Option<&RepoFactsSummary>,
    opts: &AiContextRenderOptions,
) -> AiContextJson {
    let budget_chars = opts.budget_tokens.saturating_mul(4);

    // Focus-filtered findings, ordered by risk so truncation drops the least
    // important first.
    let mut ordered: Vec<&Finding> = summary
        .artifacts
        .findings
        .iter()
        .filter(|finding| {
            opts.focus
                .as_ref()
                .is_none_or(|focus| focus.matches(&finding.category))
        })
        .collect();
    ordered.sort_by(|a, b| {
        crate::risk::compare_findings(a, b)
            .then_with(|| finding_location_key(a).cmp(&finding_location_key(b)))
    });

    let findings_total = ordered.len();
    let plan = ordered_plan_clusters(summary, opts.focus.as_ref());
    let plan_total = plan.len();

    let mut doc = AiContextJson {
        schema_version: AI_CONTEXT_JSON_SCHEMA_VERSION,
        project: project_name(summary),
        focus: focus_label(&opts.focus),
        budget_tokens: opts.budget_tokens,
        approx_tokens: 0,
        truncated: false,
        findings_total,
        findings_included: 0,
        findings_omitted: findings_total,
        plan_clusters_total: plan_total,
        plan_clusters_included: 0,
        // Risk summary counts the whole focus-filtered set, not just what fits.
        risk: risk_json(&ordered),
        facts: facts_summary.map(facts_json),
        findings: Vec::new(),
        plan: Vec::new(),
    };

    // Measure the envelope (risk + facts + counts) with empty collections, then
    // fill within the remaining budget — pretty bytes, matching the real output.
    let base_cost = pretty_cost(&doc);
    let mut used = base_cost;
    let findings_ceiling =
        base_cost + budget_chars.saturating_sub(base_cost) * FINDINGS_BUDGET_SHARE / 100;

    for finding in &ordered {
        let json = finding_json(finding);
        let cost = pretty_cost(&json) + 4; // +array comma/indentation
        if !doc.findings.is_empty() && used + cost > findings_ceiling {
            break;
        }
        used += cost;
        doc.findings.push(json);
    }

    for cluster in plan {
        let cost = pretty_cost(&cluster) + 4;
        if used + cost > budget_chars {
            break;
        }
        used += cost;
        doc.plan.push(cluster);
    }

    // The per-item estimate under-counts nested array indentation, so verify
    // against the real pretty encoding and trim until it genuinely fits — plan
    // clusters first (derived data), then the lowest-risk findings, always
    // keeping at least one finding for signal.
    while pretty_cost(&doc) > budget_chars {
        if doc.plan.pop().is_some() {
            continue;
        }
        if doc.findings.len() > 1 {
            doc.findings.pop();
            continue;
        }
        break;
    }

    doc.findings_included = doc.findings.len();
    doc.findings_omitted = findings_total.saturating_sub(doc.findings.len());
    doc.plan_clusters_included = doc.plan.len();
    doc.truncated = doc.findings_omitted > 0 || doc.plan_clusters_included < plan_total;
    doc
}

fn pretty_cost<T: Serialize>(value: &T) -> usize {
    serde_json::to_string_pretty(value)
        .map(|rendered| rendered.len())
        .unwrap_or(0)
}

fn focus_label(focus: &Option<AiFocusCategory>) -> &'static str {
    match focus {
        None | Some(AiFocusCategory::All) => "all",
        Some(AiFocusCategory::Security) => "security",
        Some(AiFocusCategory::Architecture) => "architecture",
        Some(AiFocusCategory::Quality) => "quality",
        Some(AiFocusCategory::Framework) => "framework",
    }
}

fn risk_json(findings: &[&Finding]) -> RiskSummaryJson {
    let count = |severity: Severity| findings.iter().filter(|f| f.severity == severity).count();
    let label = super::header::risk_level(findings);
    RiskSummaryJson {
        level: machine_level(label),
        label: label.to_string(),
        total: findings.len(),
        critical: count(Severity::Critical),
        high: count(Severity::High),
        medium: count(Severity::Medium),
        low: count(Severity::Low),
        info: count(Severity::Info),
    }
}

/// `🟡 MODERATE` -> `moderate`. The label is always `<emoji> WORD`.
fn machine_level(label: &str) -> String {
    label
        .split_whitespace()
        .last()
        .unwrap_or("clean")
        .to_ascii_lowercase()
}

fn facts_json(facts: &RepoFactsSummary) -> FactsJson {
    FactsJson {
        total_files: facts.total_files,
        files_with_language: facts.files_with_language,
        total_non_empty_lines: facts.total_non_empty_lines,
        languages: facts
            .languages
            .iter()
            .take(super::repo_facts::MAX_LANGUAGES)
            .map(|language| LanguageJson {
                language: language.language.clone(),
                files: language.files,
                non_empty_lines: language.non_empty_lines,
            })
            .collect(),
        diagnostics_count: facts.diagnostics_count,
    }
}

fn finding_json(finding: &Finding) -> FindingJson {
    FindingJson {
        id: finding.id.clone(),
        rule_id: finding.rule_id.clone(),
        category: finding.category.label(),
        severity: finding.severity.label(),
        confidence: finding.confidence.label(),
        title: finding.title.clone(),
        description: finding.description.clone(),
        recommendation: finding.recommendation_or_default().to_string(),
        docs_url: finding.docs_url.clone(),
        workspace_package: finding.workspace_package.clone(),
        risk: FindingRiskJson {
            score: finding.risk.score,
            priority: finding.risk.priority.label(),
            signals: finding
                .risk
                .signals
                .iter()
                .map(|signal| RiskSignalJson {
                    id: signal.id.clone(),
                    label: signal.label.clone(),
                    weight: signal.weight,
                    reason: signal.reason.clone(),
                })
                .collect(),
        },
        evidence: finding
            .evidence
            .iter()
            .map(|evidence| EvidenceJson {
                path: evidence.path.display().to_string(),
                line_start: evidence.line_start,
                line_end: evidence.line_end,
                snippet: evidence.snippet.clone(),
            })
            .collect(),
    }
}

#[cfg(test)]
mod tests;
