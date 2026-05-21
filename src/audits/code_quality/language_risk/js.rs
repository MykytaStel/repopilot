use super::is_library_boundary_path;
use crate::findings::types::Severity;
use std::path::Path;

// ── JavaScript / TypeScript ───────────────────────────────────────────────────

#[derive(Debug, Clone, Copy)]
pub(super) enum JsRiskPattern {
    ProcessExit,
    ThrowError,
}

impl JsRiskPattern {
    pub(super) const ALL: &'static [Self] = &[Self::ProcessExit, Self::ThrowError];

    pub(super) fn matches(self, trimmed: &str, path: &Path) -> bool {
        match self {
            Self::ProcessExit => trimmed.contains("process.exit("),
            Self::ThrowError => {
                trimmed.contains("throw new Error(") && is_library_boundary_path(path)
            }
        }
    }

    pub(super) fn rule_id(self) -> &'static str {
        "language.javascript.runtime-exit-risk"
    }

    pub(super) fn signal(self) -> &'static str {
        match self {
            Self::ProcessExit => "js.process-exit",
            Self::ThrowError => "js.throw-error",
        }
    }

    pub(super) fn title(self) -> &'static str {
        match self {
            Self::ProcessExit => "JavaScript process.exit usage",
            Self::ThrowError => "Generic JavaScript error at library boundary",
        }
    }

    pub(super) fn context_label(self) -> &'static str {
        match self {
            Self::ProcessExit => "JavaScript process exit call",
            Self::ThrowError => "JavaScript generic thrown error",
        }
    }

    pub(super) fn recommendation(self) -> &'static str {
        match self {
            Self::ProcessExit => {
                "Keep process exits at a CLI boundary and return errors from reusable modules."
            }
            Self::ThrowError => {
                "Prefer typed errors or actionable error messages at reusable package boundaries."
            }
        }
    }

    pub(super) fn base_severity(self) -> Severity {
        match self {
            Self::ProcessExit => Severity::High,
            Self::ThrowError => Severity::Medium,
        }
    }
}
