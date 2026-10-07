use crate::findings::types::Confidence;
use crate::review::signals::behavioral::BehavioralKind;
use crate::review::signals::tiered::ConfidenceTier;

pub(super) fn behavioral_kind(kind: BehavioralKind) -> &'static str {
    use BehavioralKind::*;
    match kind {
        NetworkCallAdded => "behavioral.network-call-added",
        SubprocessAdded => "behavioral.subprocess-added",
        FsWriteAdded => "behavioral.fs-write-added",
        EnvVarIntroduced => "behavioral.env-var-introduced",
        DependencyImportAdded => "behavioral.dependency-import-added",
        MigrationAdded => "behavioral.migration-added",
        RawSqlAdded => "behavioral.raw-sql-added",
        ErrorHandlingRemoved => "behavioral.error-handling-removed",
        TestDeletedOrEmptied => "behavioral.test-deleted-or-emptied",
        AuthCheckRemoved => "behavioral.auth-check-removed",
        QuietFallbackIntroduced => "behavioral.quiet-fallback-introduced",
    }
}

pub(super) fn behavioral_confidence(kind: BehavioralKind, coarse: bool) -> Confidence {
    if coarse {
        Confidence::Low
    } else if kind == BehavioralKind::QuietFallbackIntroduced {
        Confidence::Medium
    } else {
        Confidence::High
    }
}

pub(super) fn behavioral_tier(
    kind: BehavioralKind,
    in_access_boundary: bool,
    coarse: bool,
) -> ConfidenceTier {
    use BehavioralKind::*;
    if coarse {
        return ConfidenceTier::LargeDiffOrNoise;
    }
    match kind {
        SubprocessAdded | EnvVarIntroduced | MigrationAdded | ErrorHandlingRemoved
        | TestDeletedOrEmptied | AuthCheckRemoved => ConfidenceTier::DefinitelySensitive,
        NetworkCallAdded if in_access_boundary => ConfidenceTier::DefinitelySensitive,
        NetworkCallAdded
        | FsWriteAdded
        | DependencyImportAdded
        | RawSqlAdded
        | QuietFallbackIntroduced => ConfidenceTier::MaybeSensitive,
    }
}

pub(super) fn behavioral_headline(kind: BehavioralKind) -> &'static str {
    use BehavioralKind::*;
    match kind {
        NetworkCallAdded => "network call added",
        SubprocessAdded => "subprocess/exec added",
        FsWriteAdded => "filesystem write added",
        EnvVarIntroduced => "env var introduced",
        DependencyImportAdded => "dependency import added",
        MigrationAdded => "migration added",
        RawSqlAdded => "raw SQL added",
        ErrorHandlingRemoved => "error handling removed",
        TestDeletedOrEmptied => "test deleted or emptied",
        AuthCheckRemoved => "auth check removed",
        QuietFallbackIntroduced => "quiet fallback introduced",
    }
}
