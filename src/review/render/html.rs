use super::helpers::verification_proof_summary;
use super::html_assets::{SCRIPT, STYLE};
use crate::baseline::gate::CiGateResult;
use crate::review::ReviewSignalGateResult;
use crate::review::model::ReviewReport;
use crate::review::proof::{ChangeProof, derive_change_proof_from_review};
use crate::review::readiness::{MergeReadinessRecord, derive_readiness};
#[path = "html_sections.rs"]
mod html_sections;

pub fn render_review_html(
    report: &ReviewReport,
    ci_gate: Option<&CiGateResult>,
    review_gate: Option<&ReviewSignalGateResult>,
) -> String {
    let readiness = derive_readiness(
        report,
        ci_gate,
        review_gate,
        report.summary.artifacts.risk_delta.as_ref(),
    );
    let proof = derive_change_proof_from_review(report, &readiness);
    let verdict_class = proof.verdict.label().to_ascii_lowercase().replace(' ', "-");
    format!(
        r#"<!DOCTYPE html>
<html lang="en">
<head><meta charset="UTF-8"><meta name="viewport" content="width=device-width, initial-scale=1">
<title>RepoPilot Review Report</title><style>{STYLE}</style></head>
<body><main>
<header>
  <h1>RepoPilot Review Report</h1>
  <p class="meta">Path: <code>{path}</code></p>
  <p class="meta">Git root: <code>{root}</code></p>
</header>
{proof_card}
{change_map}
{impact}
{signals}
{verification}
{findings}
<script>{SCRIPT}</script>
</main></body></html>"#,
        path = escape(&report.summary.root_path.to_string_lossy()),
        root = escape(&report.repo_root.to_string_lossy()),
        proof_card = render_proof_card(
            report,
            &readiness,
            &proof,
            verdict_class,
            ci_gate,
            review_gate,
        ),
        change_map = html_sections::render_change_map(report, &proof),
        impact = html_sections::render_impact(report),
        signals = html_sections::render_signals(&report.tiered_signals),
        verification = html_sections::render_verification(report, &proof),
        findings = html_sections::render_findings(report),
    )
}

fn render_proof_card(
    report: &ReviewReport,
    readiness: &MergeReadinessRecord,
    proof: &ChangeProof,
    verdict_class: String,
    ci_gate: Option<&CiGateResult>,
    review_gate: Option<&ReviewSignalGateResult>,
) -> String {
    let limits = if proof.coverage.excluded_files > 0 || proof.coverage.unsupported_files > 0 {
        format!(
            "<ul class=\"limits\"><li>{} excluded, {} unsupported file(s)</li></ul>",
            proof.coverage.excluded_files, proof.coverage.unsupported_files
        )
    } else {
        String::new()
    };
    let readiness_limits = readiness
        .limitations
        .iter()
        .take(5)
        .map(|limitation| format!("<li>{}</li>", escape(limitation)))
        .collect::<Vec<_>>()
        .join("");
    let readiness_limits = if readiness_limits.is_empty() {
        String::new()
    } else {
        format!("<h3>Limitations</h3><ul class=\"limits\">{readiness_limits}</ul>")
    };
    let reasons = proof
        .reasons
        .iter()
        .take(5)
        .map(|reason| format!("<li>{}</li>", escape(&reason.message)))
        .collect::<Vec<_>>()
        .join("");
    let reasons = if reasons.is_empty() {
        String::new()
    } else {
        format!("<h3>Why this verdict</h3><ul class=\"reasons\">{reasons}</ul>")
    };
    let next_action = next_action(proof);
    let gate = gate_label(ci_gate, review_gate);
    format!(
        r#"<section class="proof-card verdict-{verdict_class}" id="proof-card" aria-labelledby="proof-heading">
  <div class="proof-header"><h2 id="proof-heading">Proof Card</h2><span class="badge {verdict_class}">{verdict}</span></div>
  <div class="proof-grid">
    <dl class="metric"><dt>Change proof</dt><dd><span class="badge {verdict_class}">{verdict}</span></dd></dl>
    <dl class="metric"><dt>Merge readiness</dt><dd><span class="badge {readiness_class}">{readiness}</span></dd></dl>
    <dl class="metric"><dt>Proof scope</dt><dd>{analyzed}/{requested} file(s) analyzed</dd></dl>
    <dl class="metric"><dt>Verification proof</dt><dd>{verification}</dd></dl>
    <dl class="metric"><dt>Gate</dt><dd>{gate}</dd></dl>
  </div>
  <div class="next-action"><strong>Next action:</strong> {next_action}</div>
  {limits}{readiness_limits}{reasons}
</section>"#,
        verdict = proof.verdict.label(),
        readiness = readiness.verdict.label(),
        readiness_class = readiness.verdict.label(),
        analyzed = proof.coverage.analyzed_files,
        requested = proof.coverage.requested_files,
        verification = escape(&verification_proof_summary(report, proof.obligations)),
        gate = escape(&gate),
        next_action = escape(next_action),
    )
}

fn next_action(proof: &ChangeProof) -> &'static str {
    match proof.verdict {
        crate::review::proof::ChangeProofVerdict::Broken => {
            "Inspect the broken contract and its listed consumer before merge."
        }
        crate::review::proof::ChangeProofVerdict::Review => {
            "Review the listed evidence, close the proof limits, or run the required checks."
        }
        crate::review::proof::ChangeProofVerdict::Verified => {
            "Proceed with the normal merge review; the reported scope has compatible proof."
        }
        crate::review::proof::ChangeProofVerdict::NotAssessed => {
            "Expand the analyzable scope before treating this review as evidence."
        }
    }
}

fn gate_label(
    ci_gate: Option<&CiGateResult>,
    review_gate: Option<&ReviewSignalGateResult>,
) -> String {
    let mut gates = Vec::new();
    if let Some(gate) = ci_gate {
        gates.push(format!(
            "finding gate {} ({})",
            if gate.passed() { "passed" } else { "failed" },
            gate.label()
        ));
    }
    if let Some(gate) = review_gate.filter(|gate| gate.enabled()) {
        gates.push(format!(
            "review gate {} ({}, {} failed)",
            if gate.passed() { "passed" } else { "failed" },
            gate.label(),
            gate.failed_signals
        ));
    }
    if gates.is_empty() {
        "not configured".to_string()
    } else {
        gates.join("; ")
    }
}

fn escape(value: &str) -> String {
    value
        .replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
        .replace('\'', "&#39;")
}
