use crate::findings::types::Severity;
use std::path::Path;

// ── Python ────────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Copy)]
pub(super) enum PythonRiskPattern {
    BroadExcept,
    Assert,
    NotImplemented,
}

impl PythonRiskPattern {
    pub(super) const ALL: &'static [Self] =
        &[Self::BroadExcept, Self::Assert, Self::NotImplemented];

    pub(super) fn matches(self, trimmed: &str, _path: &Path) -> bool {
        match self {
            Self::BroadExcept => {
                let normalized = trimmed.replace(' ', "");
                normalized == "except:" || normalized.starts_with("except:#")
            }
            Self::Assert => trimmed.starts_with("assert ") || trimmed.starts_with("assert("),
            Self::NotImplemented => {
                trimmed.contains("raise NotImplementedError")
                    || trimmed.contains("NotImplementedError(")
                    || trimmed == "raise NotImplementedError"
            }
        }
    }

    pub(super) fn rule_id(self) -> &'static str {
        "language.python.exception-risk"
    }

    pub(super) fn signal(self) -> &'static str {
        match self {
            Self::BroadExcept => "python.broad-except",
            Self::Assert => "python.assert",
            Self::NotImplemented => "python.not-implemented",
        }
    }

    pub(super) fn title(self) -> &'static str {
        match self {
            Self::BroadExcept => "Broad Python except handler",
            Self::Assert => "Python assert in production path",
            Self::NotImplemented => "Python NotImplementedError placeholder",
        }
    }

    pub(super) fn context_label(self) -> &'static str {
        match self {
            Self::BroadExcept => "Python broad exception handler",
            Self::Assert => "Python assert statement",
            Self::NotImplemented => "Python not-implemented placeholder",
        }
    }

    pub(super) fn recommendation(self) -> &'static str {
        match self {
            Self::BroadExcept => "Catch specific exceptions so unrelated failures are not hidden.",
            Self::Assert => {
                "Use explicit runtime validation for production invariants because asserts can be disabled."
            }
            Self::NotImplemented => {
                "Replace placeholders before production release or guard them behind explicit feature flags."
            }
        }
    }

    pub(super) fn base_severity(self) -> Severity {
        match self {
            Self::NotImplemented => Severity::High,
            _ => Severity::Medium,
        }
    }
}
