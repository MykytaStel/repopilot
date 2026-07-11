//! Canonical JSON analysis artifact emitted by `repopilot ai context --format json`.
//!
//! The Markdown brief remains optimized for a human/agent handoff. This JSON
//! surface is the stable machine contract: it reuses [`FindingRecord`] and its
//! canonical [`DecisionRecord`] instead of reconstructing a second finding DTO.
//! That keeps scan, review, SARIF, MCP, explain, and AI-context consumers aligned.
//!
//! The artifact remains budget-aware. Findings are ordered by risk and included
//! until the pretty-serialized output reaches `--budget`. The artifact explicitly
//! reports requested/used budget, truncation, included/omitted findings, included
//! verification plans, and plan-cluster counts.
//!
//! Deterministic by construction: no clock, network, randomness, or unstable map
//! ordering is used, so the artifact is suitable for golden and CI contracts.

use crate::facts::RepoFactsSummary;
use crate::findings::record::FindingRecord;
use crate::findings::types::{Finding, Severity};
use crate::output::ai_context::{AiContextRenderOptions, AiFocusCategory, project_name};
use crate::output::ai_plan::{PlanCluster, ordered_plan_clusters};
use crate::output::finding_helpers::finding_location_key;
use crate::report::schema::{REPOPILOT_VERSION, SCAN_REPORT_SCHEMA_VERSION};
use crate::scan::types::ScanSummary;
use serde::Serialize;

/// Bumped when the AI analysis artifact shape changes incompatibly.
pub const AI_CONTEXT_JSON_SCHEMA_VERSION: u32 = 3;

/// Version of the canonical RepoPilot analysis-artifact envelope.
pub const AI_CONTEXT_ARTIFACT_VERSION: u32 = 1;

/// Share of the available budget findings may consume before remediation-plan
/// clusters get a turn.
const FINDINGS_BUDGET_SHARE: usize = 70;

#[derive(Serialize)]
struct AiContextArtifact<'a> {
    schema_version: u32,
    artifact: ArtifactMetadata,
    project: String,
    focus: &'static str,
    budget: BudgetMetadata,
    summary: ArtifactSummary,
    risk: RiskSummaryJson,
    #[serde(skip_serializing_if = "Option::is_none")]
    facts: Option<FactsJson>,
    findings: Vec<FindingRecord<'a>>,
    plan: Vec<PlanCluster>,
}

#[derive(Serialize)]
struct ArtifactMetadata {
    kind: &'static str,
    version: u32,
    source: &'static str,
    report_schema_version: &'static str,
    repopilot_version: &'static str,
}

#[derive(Serialize)]
struct BudgetMetadata {
    requested_tokens: usize,
    approx_tokens: usize,
    truncated: bool,
}

#[derive(Serialize)]
struct ArtifactSummary {
    findings_total: usize,
    findings_included: usize,
    findings_omitted: usize,
    verification_plans_included: usize,
    plan_clusters_total: usize,
    plan_clusters_included: usize,
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

/// Render the canonical AI analysis artifact as pretty JSON.
///
/// Serialization failure is returned as a small JSON error document instead of
/// panicking. `approx_tokens` is calculated from the actual pretty output.
pub fn render_json(
    summary: &ScanSummary,
    facts_summary: Option<&RepoFactsSummary>,
    opts: &AiContextRenderOptions,
) -> String {
    let mut artifact = build(summary, facts_summary, opts);

    if let Ok(pretty) = serde_json::to_string_pretty(&artifact) {
        artifact.budget.approx_tokens = pretty.len() / 4;
    }

    serde_json::to_string_pretty(&artifact).unwrap_or_else(|error| {
        format!("{{\n  \"error\": \"AI analysis artifact serialization failed: {error}\"\n}}")
    })
}

fn build<'a>(
    summary: &'a ScanSummary,
    facts_summary: Option<&RepoFactsSummary>,
    opts: &AiContextRenderOptions,
) -> AiContextArtifact<'a> {
    let budget_chars = opts.budget_tokens.saturating_mul(4);

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

    ordered.sort_by(|left, right| {
        crate::risk::compare_findings(left, right)
            .then_with(|| finding_location_key(left).cmp(&finding_location_key(right)))
    });

    let findings_total = ordered.len();
    let plan = ordered_plan_clusters(summary, opts.focus.as_ref());
    let plan_clusters_total = plan.len();

    let mut artifact = AiContextArtifact {
        schema_version: AI_CONTEXT_JSON_SCHEMA_VERSION,
        artifact: ArtifactMetadata {
            kind: "repopilot-analysis",
            version: AI_CONTEXT_ARTIFACT_VERSION,
            source: "ai-context",
            report_schema_version: SCAN_REPORT_SCHEMA_VERSION,
            repopilot_version: REPOPILOT_VERSION,
        },
        project: project_name(summary),
        focus: focus_label(&opts.focus),
        budget: BudgetMetadata {
            requested_tokens: opts.budget_tokens,
            approx_tokens: 0,
            truncated: false,
        },
        summary: ArtifactSummary {
            findings_total,
            findings_included: 0,
            findings_omitted: findings_total,
            verification_plans_included: 0,
            plan_clusters_total,
            plan_clusters_included: 0,
        },
        risk: risk_json(&ordered),
        facts: facts_summary.map(facts_json),
        findings: Vec::new(),
        plan: Vec::new(),
    };

    // Measure the envelope first, then reserve 30% of the remaining budget for
    // the remediation plan. The first finding is retained as a signal floor.
    let base_cost = pretty_cost(&artifact);
    let mut used = base_cost;
    let findings_ceiling =
        base_cost + budget_chars.saturating_sub(base_cost) * FINDINGS_BUDGET_SHARE / 100;

    for finding in ordered {
        let record = FindingRecord::new(finding);
        let cost = pretty_cost(&record) + 4;
        if !artifact.findings.is_empty() && used + cost > findings_ceiling {
            break;
        }
        used += cost;
        artifact.findings.push(record);
    }

    for cluster in plan {
        let cost = pretty_cost(&cluster) + 4;
        if used + cost > budget_chars {
            break;
        }
        used += cost;
        artifact.plan.push(cluster);
    }

    // Verify the real pretty encoding. Derived plan clusters are removed first,
    // then the lowest-priority included findings, while preserving one finding.
    while pretty_cost(&artifact) > budget_chars {
        if artifact.plan.pop().is_some() {
            continue;
        }
        if artifact.findings.len() > 1 {
            artifact.findings.pop();
            continue;
        }
        break;
    }

    artifact.summary.findings_included = artifact.findings.len();
    artifact.summary.findings_omitted = findings_total.saturating_sub(artifact.findings.len());
    artifact.summary.verification_plans_included = artifact
        .findings
        .iter()
        .filter(|record| record.decision.verification_plan.is_some())
        .count();
    artifact.summary.plan_clusters_included = artifact.plan.len();
    artifact.budget.truncated = artifact.summary.findings_omitted > 0
        || artifact.summary.plan_clusters_included < plan_clusters_total;

    artifact
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
    let count = |severity: Severity| {
        findings
            .iter()
            .filter(|finding| finding.severity == severity)
            .count()
    };
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

/// `🟡 MODERATE` -> `moderate`.
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

#[cfg(test)]
mod tests;
