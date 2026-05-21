use crate::findings::types::Severity;
use std::path::Path;

// ── Java / Kotlin / C# ───────────────────────────────────────────────────────

#[derive(Debug, Clone, Copy)]
pub(super) enum ManagedRiskPattern {
    FatalException { is_csharp: bool },
    NotImplemented,
}

impl ManagedRiskPattern {
    pub(super) const JVM_PATTERNS: &'static [Self] = &[
        Self::FatalException { is_csharp: false },
        Self::NotImplemented,
    ];
    pub(super) const CSHARP_PATTERNS: &'static [Self] = &[
        Self::FatalException { is_csharp: true },
        Self::NotImplemented,
    ];

    pub(super) fn matches(self, trimmed: &str, _path: &Path) -> bool {
        match self {
            Self::FatalException { is_csharp } => {
                trimmed.contains("throw new RuntimeException(")
                    || trimmed.contains("throw new IllegalStateException(")
                    || (is_csharp && trimmed.contains("throw new Exception("))
            }
            Self::NotImplemented => {
                trimmed.contains("throw new NotImplementedException(")
                    || trimmed.contains("throw new NotImplementedError(")
                    || trimmed.contains("TODO(")
                    || trimmed.contains("TODO()")
            }
        }
    }

    pub(super) fn rule_id(self) -> &'static str {
        "language.managed.fatal-exception-risk"
    }

    pub(super) fn signal(self) -> &'static str {
        match self {
            Self::FatalException { .. } => "managed.fatal-exception",
            Self::NotImplemented => "managed.not-implemented",
        }
    }

    pub(super) fn title(self) -> &'static str {
        match self {
            Self::FatalException { .. } => "Generic fatal exception in managed code",
            Self::NotImplemented => "Not-implemented placeholder in managed code",
        }
    }

    pub(super) fn context_label(self) -> &'static str {
        match self {
            Self::FatalException { .. } => "JVM/.NET generic fatal exception",
            Self::NotImplemented => "JVM/.NET placeholder failure",
        }
    }

    pub(super) fn recommendation(self) -> &'static str {
        match self {
            Self::FatalException { .. } => {
                "Use domain-specific exception or result types when callers need precise recovery behaviour."
            }
            Self::NotImplemented => {
                "Replace placeholders before production release or isolate unfinished paths clearly."
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
