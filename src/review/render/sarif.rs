use crate::findings::provenance::FindingProvenance;
use crate::findings::types::{Evidence, Finding, FindingCategory, Severity};
use crate::output::sarif::findings_to_sarif;
use crate::review::model::ReviewReport;
use crate::review::signals::tiered::{ConfidenceTier, SignalFamily};
use std::path::PathBuf;

pub fn render_review_sarif(report: &ReviewReport) -> Result<String, serde_json::Error> {
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

    serde_json::to_string_pretty(&findings_to_sarif(&findings, &report.repo_root))
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
