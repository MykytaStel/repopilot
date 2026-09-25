use crate::review::model::ReviewReport;
use crate::review::proof::{ChangeProof, ProofCapabilityStatus, ProofCoverage};
use serde::{Deserialize, Serialize};

#[path = "coverage/reasons.rs"]
mod reasons;
#[path = "coverage/status.rs"]
mod status;
pub(crate) use status::{classify, coverage_status};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct EvidenceCoverageLimit {
    pub code: String,
    pub count: usize,
    pub message: String,
}

pub(super) fn coverage_limits(
    report: &ReviewReport,
    proof: &ChangeProof,
) -> Vec<EvidenceCoverageLimit> {
    let mut limits = Vec::new();
    add_file_scope_limits(&mut limits, report, proof);
    reasons::add_verification_limits(&mut limits, proof);
    reasons::add_reason_limits(&mut limits, report, proof);
    add_capability_limits(&mut limits, proof);
    limits.sort_by(|left, right| left.code.cmp(&right.code));
    limits.dedup_by(|left, right| left.code == right.code);
    limits
}

fn add_file_scope_limits(
    limits: &mut Vec<EvidenceCoverageLimit>,
    report: &ReviewReport,
    proof: &ChangeProof,
) {
    let coverage = &proof.coverage;
    if coverage.excluded_files > 0 {
        add_excluded_limits(limits, report, coverage.excluded_files);
    }
    if coverage.unsupported_files > 0 {
        let count = coverage.unsupported_files;
        limits.push(limit(
            "unsupported-files",
            count,
            format!(
                "{count} file{} received no supported analysis result",
                plural(count)
            ),
        ));
    }
    add_empty_scope_limit(limits, coverage);
    add_file_accounting_limit(limits, coverage);
}

fn add_empty_scope_limit(limits: &mut Vec<EvidenceCoverageLimit>, scope: &ProofCoverage) {
    if scope.requested_files == 0 {
        limits.push(limit(
            "empty-scope",
            0,
            "No files were in the requested scope.",
        ));
    } else if scope.analyzed_files == 0 {
        limits.push(limit(
            "no-analyzable-files",
            scope.requested_files,
            format!(
                "{} requested file{} received no analysis result",
                scope.requested_files,
                plural(scope.requested_files)
            ),
        ));
    }
}

fn add_file_accounting_limit(limits: &mut Vec<EvidenceCoverageLimit>, scope: &ProofCoverage) {
    let accounted_files = scope
        .analyzed_files
        .saturating_add(scope.excluded_files)
        .saturating_add(scope.unsupported_files)
        .saturating_add(scope.policy_skipped_files);
    if accounted_files != scope.requested_files {
        let count = scope.requested_files.saturating_sub(accounted_files);
        limits.push(limit(
            "unaccounted-files",
            count.max(1),
            "File accounting does not cover the full requested scope.".to_string(),
        ));
    }
}

fn add_capability_limits(limits: &mut Vec<EvidenceCoverageLimit>, proof: &ChangeProof) {
    let has_contract_limit = limits
        .iter()
        .any(|item| item.code == "unsupported-contract-coverage");
    for capability in &proof.capability_coverage {
        if capability.count == 0
            || capability.status == ProofCapabilityStatus::Assessed
            || matches!(
                capability.id.as_str(),
                "scope.excluded-files" | "scope.unsupported-files" | "verification"
            )
            || (capability.id == "contract-deltas" && has_contract_limit)
        {
            continue;
        }
        limits.push(limit(
            format!("capability-{}", capability.id),
            capability.count,
            capability.message.clone(),
        ));
    }
}

fn add_excluded_limits(
    limits: &mut Vec<EvidenceCoverageLimit>,
    report: &ReviewReport,
    excluded_files: usize,
) {
    let metrics = &report.summary.metrics;
    let reasons = [
        (
            "files-over-size-limit",
            metrics.large_files_skipped,
            "exceeded the configured size limit",
            "exceeded the configured size limit",
        ),
        (
            "binary-files-skipped",
            metrics.binary_files_skipped,
            "was identified as binary",
            "were identified as binary",
        ),
        (
            "files-over-max-files-limit",
            metrics.files_skipped_by_limit,
            "was omitted by the max-files limit",
            "were omitted by the max-files limit",
        ),
        (
            "files-repopilotignore",
            metrics.files_skipped_repopilotignore,
            "was excluded by .repopilotignore",
            "were excluded by .repopilotignore",
        ),
    ];
    let mut remaining = excluded_files;
    for (code, reported_count, singular_cause, plural_cause) in reasons {
        let count = reported_count.min(remaining);
        if count == 0 {
            continue;
        }
        append_excluded_limit(limits, code, count, singular_cause, plural_cause);
        remaining -= count;
    }
    add_unclassified_exclusion(limits, remaining);
}

fn append_excluded_limit(
    limits: &mut Vec<EvidenceCoverageLimit>,
    code: &str,
    count: usize,
    singular_cause: &str,
    plural_cause: &str,
) {
    let cause = if count == 1 {
        singular_cause
    } else {
        plural_cause
    };
    limits.push(limit(
        code,
        count,
        format!("{} file{} {cause}.", count, plural(count)),
    ));
}

fn add_unclassified_exclusion(limits: &mut Vec<EvidenceCoverageLimit>, count: usize) {
    if count == 0 {
        return;
    }
    let verb = if count == 1 { "has" } else { "have" };
    limits.push(limit(
        "excluded-files-unclassified",
        count,
        format!(
            "{} excluded file{} {verb} no recorded skip reason.",
            count,
            plural(count)
        ),
    ));
}

pub(super) fn limit(
    code: impl Into<String>,
    count: usize,
    message: impl Into<String>,
) -> EvidenceCoverageLimit {
    EvidenceCoverageLimit {
        code: code.into(),
        count,
        message: message.into(),
    }
}

pub(super) fn plural(count: usize) -> &'static str {
    if count == 1 { "" } else { "s" }
}
