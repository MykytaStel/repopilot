pub mod context;
pub mod delta;
pub mod diagnostic;
pub mod model;
pub mod storage;

pub use context::{HistoryRecordOutcome, ReceiptContext, build_receipt, record_session};
pub use delta::compare;
pub use diagnostic::{HistoryDiagnostic, HistoryDiagnosticKind};
pub use model::{
    AnalysisScope, ComparisonIdentity, ComparisonResult, ComparisonUnavailable, FindingReceipt,
    RiskDelta, RunReceipt, SeverityShift,
};
pub use storage::{
    DEFAULT_MAX_BYTES, DEFAULT_MAX_RUNS, HISTORY_FILE, HistoryLimits, HistoryLoad, HistoryStore,
    HistoryWriteError, append_run, read_all_runs, read_last_run,
};
