use serde::Serialize;

/// The category of a behavioral change.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum BehavioralKind {
    NetworkCallAdded,
    SubprocessAdded,
    FsWriteAdded,
    EnvVarIntroduced,
    DependencyImportAdded,
    MigrationAdded,
    RawSqlAdded,
    ErrorHandlingRemoved,
    TestDeletedOrEmptied,
    AuthCheckRemoved,
    QuietFallbackIntroduced,
}

/// How a behavioral signal was detected. Confidence — and therefore tiering —
/// keys off this structured source, never off the user-facing `detail` text.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum BehavioralSignalSource {
    /// Matched structurally against the parsed syntax tree.
    Ast,
    /// Matched by scanning raw diff lines because the file could not be parsed.
    CoarseFallback,
}

/// A behavioral signal detected in a changed file.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct BehavioralSignal {
    pub kind: BehavioralKind,
    pub path: String,
    pub line: usize,
    pub detail: String,
    pub source: BehavioralSignalSource,
}

impl BehavioralSignal {
    /// Whether this signal came from the coarse (non-AST) fallback, which only
    /// runs when the file couldn't be parsed. Coarse signals are hints, not
    /// confident findings, and are demoted in the tiered view.
    pub fn is_coarse(&self) -> bool {
        self.source == BehavioralSignalSource::CoarseFallback
    }
}
