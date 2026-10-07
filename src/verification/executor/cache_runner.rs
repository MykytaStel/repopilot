use super::{
    cancelled_outcome, execute_check, execute_check_with_pinned_identity, skipped_outcome,
};
use crate::scan::session::WorkspaceRevision;
use crate::verification::cache::{VerificationCache, VerificationCacheKey};
use crate::verification::model::{
    CancellationToken, VerificationExecutionEvent, VerificationOutcome,
};
use crate::verification::policy::ValidatedCheck;
use std::path::Path;
use std::time::Instant;

pub fn run_checks_observed_cached(
    checks: &[ValidatedCheck],
    evidence_paths: &[std::path::PathBuf],
    revision: &WorkspaceRevision,
    cancellation: &CancellationToken,
    cache_root: Option<&Path>,
    observer: &mut dyn FnMut(VerificationExecutionEvent),
) -> Vec<VerificationOutcome> {
    run_checks_observed_cached_inner(
        checks,
        evidence_paths,
        revision,
        cancellation,
        cache_root,
        false,
        observer,
    )
}

pub fn run_checks_observed_cached_with_pinned_identity(
    checks: &[ValidatedCheck],
    evidence_paths: &[std::path::PathBuf],
    revision: &WorkspaceRevision,
    cancellation: &CancellationToken,
    cache_root: Option<&Path>,
    observer: &mut dyn FnMut(VerificationExecutionEvent),
) -> Vec<VerificationOutcome> {
    run_checks_observed_cached_inner(
        checks,
        evidence_paths,
        revision,
        cancellation,
        cache_root,
        true,
        observer,
    )
}

fn run_checks_observed_cached_inner(
    checks: &[ValidatedCheck],
    evidence_paths: &[std::path::PathBuf],
    revision: &WorkspaceRevision,
    cancellation: &CancellationToken,
    cache_root: Option<&Path>,
    enforce_identity: bool,
    observer: &mut dyn FnMut(VerificationExecutionEvent),
) -> Vec<VerificationOutcome> {
    let mut outcomes = Vec::with_capacity(checks.len());
    let mut revision_changed = false;
    let total = checks.len();
    let cache = cache_root.map(VerificationCache::new);
    for (offset, check) in checks.iter().enumerate() {
        if cancellation.is_cancelled() {
            break;
        }
        let index = offset + 1;
        observer(VerificationExecutionEvent::Started {
            check_id: check.id().to_string(),
            index,
            total,
        });
        let outcome = outcome_for_check(
            check,
            evidence_paths,
            revision,
            cancellation,
            cache.as_ref(),
            revision_changed,
            enforce_identity,
        );
        revision_changed = !outcome.revision_compatible;
        observer(VerificationExecutionEvent::Completed {
            check_id: check.id().to_string(),
            index,
            total,
            status: outcome.status,
        });
        outcomes.push(outcome);
    }
    outcomes
}

fn outcome_for_check(
    check: &ValidatedCheck,
    evidence_paths: &[std::path::PathBuf],
    revision: &WorkspaceRevision,
    cancellation: &CancellationToken,
    cache: Option<&VerificationCache>,
    revision_changed: bool,
    enforce_identity: bool,
) -> VerificationOutcome {
    if revision_changed {
        return skipped_outcome(
            check,
            revision,
            "workspace revision changed before this check could run",
            false,
        );
    }
    if !check.is_applicable(evidence_paths.iter().map(std::path::PathBuf::as_path)) {
        return skipped_outcome(
            check,
            revision,
            "no changed or impacted path matched this check",
            true,
        );
    }
    execute_or_reuse(check, revision, cancellation, cache, enforce_identity)
}

fn execute_or_reuse(
    check: &ValidatedCheck,
    revision: &WorkspaceRevision,
    cancellation: &CancellationToken,
    cache: Option<&VerificationCache>,
    enforce_identity: bool,
) -> VerificationOutcome {
    if enforce_identity && !check.executable_identity_matches() {
        return skipped_outcome(
            check,
            revision,
            "configured executable changed before execution",
            true,
        );
    }
    if cancellation.is_cancelled() {
        return cancelled_outcome(check, revision, Instant::now());
    }
    let key = cache
        .filter(|_| check.cache_enabled())
        .and_then(|_| VerificationCacheKey::build(check, revision));
    if cancellation.is_cancelled() {
        return cancelled_outcome(check, revision, Instant::now());
    }
    if let Some(hit) =
        cache.and_then(|cache| key.as_ref().and_then(|key| cache.load(key, revision)))
    {
        if enforce_identity && !check.executable_identity_matches() {
            return skipped_outcome(
                check,
                revision,
                "configured executable changed before execution",
                true,
            );
        }
        return if cancellation.is_cancelled() {
            cancelled_outcome(check, revision, Instant::now())
        } else {
            hit
        };
    }
    let outcome = if enforce_identity {
        execute_check_with_pinned_identity(check, revision, cancellation)
    } else {
        execute_check(check, revision, cancellation)
    };
    if !cancellation.is_cancelled()
        && let (Some(cache), Some(key)) = (cache, key.as_ref())
    {
        cache.store(key, &outcome);
    }
    outcome
}
