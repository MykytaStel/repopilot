use crate::baseline::gate::CiGateResult;
use crate::findings::provenance::FindingProvenance;
use crate::findings::types::{Evidence, Finding, FindingCategory, Severity};
use crate::output::sarif::findings_to_sarif;
use crate::review::ReviewSignalGateResult;
use crate::review::derive_readiness;
use crate::review::model::ReviewReport;
use crate::review::proof::{EvidenceSummary, build_proof_receipt, derive_change_proof_from_review};
use crate::review::signals::tiered::{ConfidenceTier, SignalFamily};
use std::path::PathBuf;

pub fn render_review_sarif(report: &ReviewReport) -> Result<String, serde_json::Error> {
    render_review_sarif_with_gates(report, None, None)
}

pub fn render_review_sarif_with_gates(
    report: &ReviewReport,
    ci_gate: Option<&CiGateResult>,
    review_gate: Option<&ReviewSignalGateResult>,
) -> Result<String, serde_json::Error> {
    let mut findings = report
        .in_diff_findings()
        .into_iter()
        .cloned()
        .collect::<Vec<_>>();

    for signal in report
        .tiered_signals
        .definitely
        .iter()
        .chain(report.tiered_signals.maybe.iter())
        .filter(|signal| signal.family == SignalFamily::Taint && !signal.suppressed)
    {
        let Some(line) = signal.line_start else {
            continue;
        };
        findings.push(Finding {
            id: signal.signal_id.clone(),
            rule_id: signal.kind.clone(),
            title: signal.headline.clone(),
            description: signal
                .detail
                .clone()
                .unwrap_or_else(|| signal.headline.clone()),
            recommendation:
                "Validate the input boundary and use a safe, parameterized or allowlisted sink API."
                    .to_string(),
            category: FindingCategory::Security,
            // Read off `signal.tier`, the canonical field `taint_tier(SinkKind)`
            // already set when the signal was built — not re-derived from
            // `signal.kind`. A second string match here (`"taint.sql" | "taint.exec"
            // => High`) let this silently default a new SinkKind variant to Medium
            // even if `taint_tier` tiered it DefinitelySensitive, since nothing
            // forced the two matches to agree.
            severity: severity_for_tier(signal.tier),
            confidence: signal.confidence,
            evidence: vec![Evidence {
                path: PathBuf::from(&signal.path),
                line_start: line,
                line_end: signal.line_end,
                snippet: signal
                    .detail
                    .clone()
                    .unwrap_or_else(|| signal.headline.clone()),
            }],
            workspace_package: None,
            docs_url: None,
            provenance: FindingProvenance {
                detector: signal.provenance.detector.clone(),
                signal_source: signal.provenance.signal_source,
                rule_lifecycle: signal.provenance.lifecycle,
                analysis_scope: signal.provenance.analysis_scope,
                knowledge_decision: None,
            },
            risk: Default::default(),
        });
    }

    let readiness = derive_readiness(
        report,
        ci_gate,
        review_gate,
        report.summary.artifacts.risk_delta.as_ref(),
    );
    let proof = derive_change_proof_from_review(report, &readiness);
    let evidence = EvidenceSummary::from_review(report, &proof);
    let proof_receipt = build_proof_receipt(report, &proof);
    let mut sarif = findings_to_sarif(&findings, &report.repo_root);
    if let Some(run) = sarif.runs.first_mut() {
        run.properties.change_proof = Some(serde_json::to_value(proof)?);
        run.properties.evidence = Some(serde_json::to_value(evidence)?);
        run.properties.proof_receipt = Some(serde_json::to_value(proof_receipt)?);
        if !report.verification.is_empty() {
            run.properties.verification = Some(report.verification.clone());
        }
    }
    serde_json::to_string_pretty(&sarif)
}

/// Exhaustive so a new `ConfidenceTier` variant fails to compile here instead
/// of silently landing on a default.
fn severity_for_tier(tier: ConfidenceTier) -> Severity {
    match tier {
        ConfidenceTier::DefinitelySensitive => Severity::High,
        ConfidenceTier::MaybeSensitive => Severity::Medium,
        // Taint signals are only ever bucketed into definitely/maybe; this arm
        // exists so the match stays exhaustive if that ever changes.
        ConfidenceTier::LargeDiffOrNoise => Severity::Low,
    }
}

#[cfg(test)]
mod tests;
