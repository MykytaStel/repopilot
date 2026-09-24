//! The review header: one verdict, its reasons, and one next action first.
//! Compact output stops at the proof summary; `--detail full` adds the
//! provenance, legacy readiness, ownership, and location records.

use super::super::diagnostics;
use crate::baseline::gate::CiGateResult;
use crate::review::ReviewSignalGateResult;
use crate::review::decision::{ReviewDecision, derive_review_decision};
use crate::review::derive_readiness;
use crate::review::intent::IntentStatus;
use crate::review::model::ReviewReport;
use crate::review::ownership::OwnershipAssessment;
use crate::review::proof::{ChangeProof, EvidenceSummary, derive_change_proof_from_review};
use crate::review::readiness::MergeReadinessRecord;
use crate::review::render::helpers::{
    change_proof_headline, change_proof_policy_summary, legacy_readiness_summary,
    verification_proof_summary,
};
use std::fmt::Write;

const MAX_HEADER_REASONS: usize = 5;

pub(super) fn render(
    output: &mut String,
    report: &ReviewReport,
    ci_gate: Option<&CiGateResult>,
    review_gate: Option<&ReviewSignalGateResult>,
    full: bool,
) {
    output.push_str("RepoPilot Review\n\n");
    let readiness = derive_readiness(
        report,
        ci_gate,
        review_gate,
        report.summary.artifacts.risk_delta.as_ref(),
    );
    let proof = derive_change_proof_from_review(report, &readiness);
    let evidence = EvidenceSummary::from_review(report, &proof);
    let decision = derive_review_decision(report, &proof, &readiness, ci_gate, review_gate);

    render_verdict(output, &decision, &proof);
    render_proof_summary(output, report, &proof, &evidence);
    if full {
        render_proof_details(output, report, &proof, &evidence, &readiness);
    } else {
        render_compact_context(output, &proof, &readiness);
    }
    diagnostics::render_console(output, &report.summary, !full);
    render_gates(output, ci_gate, review_gate);
    output.push('\n');
    render_counts(output, report, full);
}

fn render_verdict(output: &mut String, decision: &ReviewDecision, proof: &ChangeProof) {
    let _ = writeln!(
        output,
        "Decision: {} (Change Proof: {})",
        decision.verdict.label(),
        proof.verdict.label()
    );
    let _ = writeln!(output, "Meaning: {}", decision.meaning);
    if !proof.reasons.is_empty() {
        output.push_str("Reasons:\n");
        for reason in proof.reasons.iter().take(MAX_HEADER_REASONS) {
            let _ = writeln!(output, "  - {}", reason.message);
        }
        if proof.reasons.len() > MAX_HEADER_REASONS {
            let _ = writeln!(
                output,
                "  ... {} more reason(s) in --format json",
                proof.reasons.len() - MAX_HEADER_REASONS
            );
        }
    }
    let _ = writeln!(output, "Next action: {}", decision.next_action);
}

fn render_proof_summary(
    output: &mut String,
    report: &ReviewReport,
    proof: &ChangeProof,
    evidence: &EvidenceSummary,
) {
    let _ = writeln!(output, "Evidence scope: {}", evidence.scope_line());
    let _ = writeln!(
        output,
        "Verification proof: {}",
        verification_proof_summary(report, proof.obligations)
    );
    let _ = writeln!(
        output,
        "Proof policy: {}",
        change_proof_policy_summary(report)
    );
}

/// Compact output only mentions intent and ownership when they carry signal.
fn render_compact_context(
    output: &mut String,
    proof: &ChangeProof,
    readiness: &MergeReadinessRecord,
) {
    if proof.intent_drift.status != IntentStatus::NotSupplied {
        render_intent(output, proof);
    }
    if readiness.ownership.assessment != OwnershipAssessment::NotConfigured {
        render_ownership(output, readiness);
    }
}

fn render_proof_details(
    output: &mut String,
    report: &ReviewReport,
    proof: &ChangeProof,
    evidence: &EvidenceSummary,
    readiness: &MergeReadinessRecord,
) {
    let _ = writeln!(output, "Evidence class: {}", evidence.class.label());
    let _ = writeln!(
        output,
        "Evidence provenance: {}",
        evidence.provenance_line()
    );
    let _ = writeln!(output, "Why: {}", change_proof_headline(report, proof));
    render_intent(output, proof);
    let _ = writeln!(
        output,
        "Proof scope: {}/{} file(s) analyzed; obligations: {}/{} satisfied",
        proof.coverage.analyzed_files,
        proof.coverage.requested_files,
        proof.obligations.satisfied,
        proof.obligations.applicable,
    );
    if proof.coverage.excluded_files > 0 || proof.coverage.unsupported_files > 0 {
        let _ = writeln!(
            output,
            "Proof limits: {} excluded, {} unsupported file(s)",
            proof.coverage.excluded_files, proof.coverage.unsupported_files
        );
    }
    let _ = writeln!(
        output,
        "Legacy merge readiness: {}",
        legacy_readiness_summary(report, readiness).to_uppercase()
    );
    render_ownership(output, readiness);
    render_location(output, report);
}

fn render_intent(output: &mut String, proof: &ChangeProof) {
    let _ = writeln!(
        output,
        "Intent drift: {}",
        proof.intent_drift.status.label()
    );
    if proof.intent_drift.is_drifted() {
        let _ = writeln!(
            output,
            "Intent limits: {} unexpected path(s), {} unexpected contract family(ies), {} critical-path mismatch(es), {} unselected check(s)",
            proof.intent_drift.unexpected_paths.len(),
            proof.intent_drift.unexpected_contract_families.len(),
            proof.intent_drift.unexpected_critical_paths.len(),
            proof.intent_drift.missing_verification.len(),
        );
    }
}

fn render_ownership(output: &mut String, readiness: &MergeReadinessRecord) {
    let ownership = &readiness.ownership;
    match ownership.assessment {
        OwnershipAssessment::Resolved => output.push_str("Ownership: resolved\n"),
        OwnershipAssessment::ConfiguredButUnmatched => {
            let _ = writeln!(
                output,
                "Ownership: configured, {} path(s) unmatched",
                ownership.unowned_paths.len()
            );
        }
        OwnershipAssessment::NotConfigured => {
            output.push_str("Ownership: not configured (not assessed)\n")
        }
    }
    if !ownership.suggested_owners.is_empty() {
        let owners = ownership
            .suggested_owners
            .iter()
            .map(|owner| owner.value.as_str())
            .collect::<Vec<_>>()
            .join(", ");
        let _ = writeln!(output, "Suggested owners: {owners}");
    }
    if !ownership.unowned_paths.is_empty() {
        let _ = writeln!(
            output,
            "Unowned changed/impacted paths: {}",
            ownership.unowned_paths.len()
        );
    }
}

fn render_location(output: &mut String, report: &ReviewReport) {
    let _ = writeln!(output, "Path: {}", report.summary.root_path.display());
    let _ = writeln!(output, "Git root: {}", report.repo_root.display());
    match &report.baseline_path {
        Some(path) => {
            let _ = writeln!(output, "Baseline: {}", path.display());
        }
        None => output.push_str("Baseline: none (all findings treated as new)\n"),
    }
    if let Some(feedback) = &report.summary.local_feedback {
        let _ = writeln!(
            output,
            "Local feedback: {} finding + {} review suppression(s) loaded, {} finding(s) + {} review signal(s) suppressed",
            feedback.suppressions_loaded,
            feedback.review_suppressions_loaded,
            feedback.suppressed_findings_count,
            feedback.suppressed_review_signals_count,
        );
    }
}

fn render_gates(
    output: &mut String,
    ci_gate: Option<&CiGateResult>,
    review_gate: Option<&ReviewSignalGateResult>,
) {
    match ci_gate {
        Some(gate) => {
            let status = if gate.passed() { "passed" } else { "failed" };
            let _ = writeln!(output, "CI gate: {status} ({})", gate.label());
        }
        None => output.push_str("CI gate: not configured\n"),
    }
    match review_gate {
        Some(gate) if gate.enabled() => {
            let status = if gate.passed() { "passed" } else { "failed" };
            let _ = writeln!(
                output,
                "Review gate: {status} ({}, {} signal(s))",
                gate.label(),
                gate.failed_signals
            );
        }
        Some(_) => output.push_str("Review gate: disabled\n"),
        None => output.push_str("Review gate: not configured\n"),
    }
}

fn render_counts(output: &mut String, report: &ReviewReport, full: bool) {
    let _ = writeln!(output, "Changed files: {}", report.changed_files.len());
    let _ = writeln!(output, "In-diff findings: {}", report.in_diff_count());
    let _ = writeln!(
        output,
        "Out-of-diff findings: {}",
        report.out_of_diff_count()
    );
    if full {
        let _ = writeln!(
            output,
            "New in-diff findings: {}",
            report.new_in_diff_count()
        );
        let _ = writeln!(
            output,
            "Existing in-diff findings: {}",
            report.existing_in_diff_count()
        );
    }
}
