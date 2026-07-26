use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum HistoryDiagnosticKind {
    InvalidRecord,
    TruncatedRecord,
    UnsupportedSchema,
    ReadFailed,
    WriteFailed,
}

impl HistoryDiagnosticKind {
    pub fn code(self) -> &'static str {
        match self {
            Self::InvalidRecord => "invalid-record",
            Self::TruncatedRecord => "truncated-record",
            Self::UnsupportedSchema => "unsupported-schema",
            Self::ReadFailed => "read-failed",
            Self::WriteFailed => "write-failed",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct HistoryDiagnostic {
    pub kind: HistoryDiagnosticKind,
    pub message: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub line: Option<usize>,
}

impl HistoryDiagnostic {
    pub(super) fn at_line(
        kind: HistoryDiagnosticKind,
        line: usize,
        message: impl Into<String>,
    ) -> Self {
        Self {
            kind,
            message: message.into(),
            line: Some(line),
        }
    }

    pub(super) fn read_failed(message: impl Into<String>) -> Self {
        Self {
            kind: HistoryDiagnosticKind::ReadFailed,
            message: message.into(),
            line: None,
        }
    }

    pub(super) fn write_failed(message: impl Into<String>) -> Self {
        Self {
            kind: HistoryDiagnosticKind::WriteFailed,
            message: message.into(),
            line: None,
        }
    }
}
