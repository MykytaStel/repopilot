//! JSON form of the `ai context` handoff. The default Markdown brief is for a
//! human skimming; agents that parse structured data get the same facts without
//! Markdown parsing — closing the asymmetry with the JSON-returning MCP tools.
//!
//! It mirrors the Markdown sections as data: project + risk summary, optional
//! repository facts, focus-filtered findings, and the P0–P3 remediation plan.
//! Deterministic (no clock/rand/net), so it can be golden-tested.

use crate::facts::RepoFactsSummary;
use crate::findings::types::{Finding, Severity};
use crate::output::ai_context::{AiContextRenderOptions, AiFocusCategory, project_name};
use crate::output::ai_plan::{PlanCluster, ordered_plan_clusters};
use crate::scan::types::ScanSummary;
use serde::Serialize;

/// Bumped when the JSON shape changes in a breaking way.
pub const AI_CONTEXT_JSON_SCHEMA_VERSION: u32 = 1;

#[derive(Serialize)]
struct AiContextJson {
    schema_version: u32,
    project: String,
    focus: &'static str,
    budget_tokens: usize,
    approx_tokens: usize,
    risk: RiskJson,
    #[serde(skip_serializing_if = "Option::is_none")]
    facts: Option<FactsJson>,
    findings: Vec<FindingJson>,
    plan: Vec<PlanCluster>,
}

#[derive(Serialize)]
struct RiskJson {
    level: &'static str,
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
    rule_id: String,
    category: &'static str,
    severity: &'static str,
    confidence: &'static str,
    priority: &'static str,
    path: String,
    line: usize,
    title: String,
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
    // Fill the approximate token count from the compact encoding (1 token ≈ 4
    // chars), matching the Markdown footer's convention.
    if let Ok(compact) = serde_json::to_string(&doc) {
        doc.approx_tokens = compact.len() / 4;
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

    AiContextJson {
        schema_version: AI_CONTEXT_JSON_SCHEMA_VERSION,
        project: project_name(summary),
        focus: focus_label(&opts.focus),
        budget_tokens: opts.budget_tokens,
        approx_tokens: 0,
        risk: risk_json(&findings),
        facts: facts_summary.map(facts_json),
        findings: findings
            .iter()
            .map(|finding| finding_json(finding))
            .collect(),
        plan: ordered_plan_clusters(summary, opts.focus.as_ref()),
    }
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

fn risk_json(findings: &[&Finding]) -> RiskJson {
    let count = |severity: Severity| findings.iter().filter(|f| f.severity == severity).count();
    RiskJson {
        level: super::header::risk_level(findings),
        total: findings.len(),
        critical: count(Severity::Critical),
        high: count(Severity::High),
        medium: count(Severity::Medium),
        low: count(Severity::Low),
        info: count(Severity::Info),
    }
}

fn facts_json(facts: &RepoFactsSummary) -> FactsJson {
    FactsJson {
        total_files: facts.total_files,
        files_with_language: facts.files_with_language,
        total_non_empty_lines: facts.total_non_empty_lines,
        languages: facts
            .languages
            .iter()
            .take(8)
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
    let evidence = finding.evidence.first();
    FindingJson {
        rule_id: finding.rule_id.clone(),
        category: finding.category.label(),
        severity: finding.severity.label(),
        confidence: finding.confidence.label(),
        priority: finding.risk.priority.label(),
        path: evidence
            .map(|evidence| evidence.path.display().to_string())
            .unwrap_or_default(),
        line: evidence.map(|evidence| evidence.line_start).unwrap_or(0),
        title: finding.title.clone(),
    }
}
