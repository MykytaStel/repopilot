use crate::findings::types::Severity;
use std::path::Path;

// ── Go ────────────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Copy)]
pub(super) enum GoRiskPattern {
    Panic,
    LogFatal,
    OsExit,
}

impl GoRiskPattern {
    pub(super) const ALL: &'static [Self] = &[Self::Panic, Self::LogFatal, Self::OsExit];

    pub(super) fn matches(self, trimmed: &str, _path: &Path) -> bool {
        match self {
            Self::Panic => trimmed.contains("panic("),
            Self::LogFatal => trimmed.contains("log.Fatal(") || trimmed.contains("log.Fatalf("),
            Self::OsExit => trimmed.contains("os.Exit("),
        }
    }

    pub(super) fn rule_id(self) -> &'static str {
        "language.go.panic-exit-risk"
    }

    pub(super) fn signal(self) -> &'static str {
        match self {
            Self::Panic => "go.panic",
            Self::LogFatal => "go.log-fatal",
            Self::OsExit => "go.os-exit",
        }
    }

    pub(super) fn title(self) -> &'static str {
        match self {
            Self::Panic => "Risky Go panic usage",
            Self::LogFatal => "Risky Go log.Fatal usage",
            Self::OsExit => "Risky Go os.Exit usage",
        }
    }

    pub(super) fn context_label(self) -> &'static str {
        match self {
            Self::Panic => "Go panic call",
            Self::LogFatal => "Go fatal logging call",
            Self::OsExit => "Go process exit call",
        }
    }

    pub(super) fn recommendation(self) -> &'static str {
        match self {
            Self::Panic => {
                "Return an error from reusable Go code and let the caller decide how to recover."
            }
            Self::LogFatal => {
                "Use returned errors outside the narrow CLI boundary so libraries remain recoverable."
            }
            Self::OsExit => {
                "Centralise process exits at the CLI entrypoint and return errors elsewhere."
            }
        }
    }

    pub(super) fn base_severity(self) -> Severity {
        Severity::Medium
    }
}
