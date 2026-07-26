use crate::history::diagnostic::{HistoryDiagnostic, HistoryDiagnosticKind};
use crate::history::model::{ComparisonIdentity, HISTORY_SCHEMA_VERSION, RunReceipt};
use std::fmt;
use std::fs::{self, File, OpenOptions, TryLockError};
use std::io::{self, Write};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::{Duration, Instant};

pub const DEFAULT_MAX_RUNS: usize = 50;
pub const DEFAULT_MAX_BYTES: u64 = 5 * 1024 * 1024;
pub const HISTORY_DIR: &str = ".repopilot/history";
pub const HISTORY_FILE: &str = ".repopilot/history/runs.jsonl";
const LOCK_FILE: &str = ".repopilot/history/runs.lock";
const BACKUP_FILE: &str = ".repopilot/history/runs.backup.jsonl";
static TEMP_COUNTER: AtomicU64 = AtomicU64::new(0);

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct HistoryLimits {
    pub max_runs: usize,
    pub max_bytes: u64,
}

impl Default for HistoryLimits {
    fn default() -> Self {
        Self {
            max_runs: DEFAULT_MAX_RUNS,
            max_bytes: DEFAULT_MAX_BYTES,
        }
    }
}

#[derive(Debug, Default)]
pub struct HistoryLoad {
    pub receipts: Vec<RunReceipt>,
    pub diagnostics: Vec<HistoryDiagnostic>,
}

#[derive(Debug)]
pub enum HistoryWriteError {
    Busy,
    Io(io::Error),
    Json(serde_json::Error),
}

impl fmt::Display for HistoryWriteError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Busy => write!(formatter, "history ledger is busy"),
            Self::Io(error) => write!(formatter, "history IO failed: {error}"),
            Self::Json(error) => write!(formatter, "history serialization failed: {error}"),
        }
    }
}

impl std::error::Error for HistoryWriteError {}

pub struct HistoryStore {
    root: PathBuf,
    limits: HistoryLimits,
}

impl HistoryStore {
    pub fn new(root: &Path, limits: HistoryLimits) -> Self {
        Self {
            root: root.to_path_buf(),
            limits,
        }
    }

    pub fn load(&self) -> HistoryLoad {
        load_path(&history_file_path(&self.root))
    }

    pub fn newest_compatible(&self, identity: &ComparisonIdentity) -> Option<RunReceipt> {
        self.load()
            .receipts
            .into_iter()
            .rev()
            .find(|receipt| receipt.comparison == *identity)
    }

    pub fn record(&self, receipt: &RunReceipt) -> Result<(), HistoryWriteError> {
        let dir = self.root.join(HISTORY_DIR);
        fs::create_dir_all(&dir).map_err(HistoryWriteError::Io)?;
        let _lock = LedgerLock::acquire(&self.root)?;
        let mut receipts = self.load().receipts;
        receipts.push(receipt.clone());
        let rendered = render_bounded(receipts, self.limits)?;
        replace_ledger(&self.root, &rendered)
    }
}

pub fn history_file_path(root: &Path) -> PathBuf {
    root.join(HISTORY_FILE)
}

pub fn read_last_run(root: &Path) -> Option<RunReceipt> {
    read_all_runs(root).pop()
}

pub fn read_all_runs(root: &Path) -> Vec<RunReceipt> {
    HistoryStore::new(root, HistoryLimits::default())
        .load()
        .receipts
}

pub fn append_run(root: &Path, receipt: &RunReceipt, max_runs: usize) -> io::Result<()> {
    HistoryStore::new(
        root,
        HistoryLimits {
            max_runs,
            max_bytes: DEFAULT_MAX_BYTES,
        },
    )
    .record(receipt)
    .map_err(io::Error::other)
}

fn load_path(path: &Path) -> HistoryLoad {
    let bytes = match fs::read(path) {
        Ok(bytes) => bytes,
        Err(error) if error.kind() == io::ErrorKind::NotFound => return HistoryLoad::default(),
        Err(error) => {
            return HistoryLoad {
                diagnostics: vec![HistoryDiagnostic::read_failed(error.to_string())],
                receipts: Vec::new(),
            };
        }
    };
    parse_lines(&bytes)
}

fn parse_lines(bytes: &[u8]) -> HistoryLoad {
    let mut loaded = HistoryLoad::default();
    let ends_with_newline = bytes.ends_with(b"\n");
    let text = String::from_utf8_lossy(bytes);
    let lines = text.lines().collect::<Vec<_>>();
    for (index, line) in lines.iter().enumerate() {
        if line.trim().is_empty() {
            continue;
        }
        match serde_json::from_str::<RunReceipt>(line) {
            Ok(receipt) if receipt.schema_version == HISTORY_SCHEMA_VERSION => {
                loaded.receipts.push(receipt);
            }
            Ok(receipt) => loaded.diagnostics.push(HistoryDiagnostic::at_line(
                HistoryDiagnosticKind::UnsupportedSchema,
                index + 1,
                format!("unsupported history schema {}", receipt.schema_version),
            )),
            Err(error) => {
                let is_truncated = index + 1 == lines.len() && !ends_with_newline;
                loaded.diagnostics.push(HistoryDiagnostic::at_line(
                    if is_truncated {
                        HistoryDiagnosticKind::TruncatedRecord
                    } else {
                        HistoryDiagnosticKind::InvalidRecord
                    },
                    index + 1,
                    error.to_string(),
                ));
            }
        }
    }
    loaded
}

fn render_bounded(
    receipts: Vec<RunReceipt>,
    limits: HistoryLimits,
) -> Result<Vec<u8>, HistoryWriteError> {
    let mut kept = Vec::new();
    let mut used = 0_u64;
    for receipt in receipts.into_iter().rev().take(limits.max_runs) {
        let mut line = serde_json::to_vec(&receipt).map_err(HistoryWriteError::Json)?;
        line.push(b'\n');
        if used.saturating_add(line.len() as u64) > limits.max_bytes {
            break;
        }
        used += line.len() as u64;
        kept.push(line);
    }
    kept.reverse();
    Ok(kept.into_iter().flatten().collect())
}

fn replace_ledger(root: &Path, contents: &[u8]) -> Result<(), HistoryWriteError> {
    let primary = history_file_path(root);
    let backup = root.join(BACKUP_FILE);
    let temporary = unique_temporary(root);
    write_staged(&temporary, contents)?;
    if primary.exists() {
        let _ = fs::remove_file(&backup);
        fs::rename(&primary, &backup).map_err(HistoryWriteError::Io)?;
    }
    if let Err(error) = fs::rename(&temporary, &primary) {
        restore_backup(&primary, &backup);
        let _ = fs::remove_file(&temporary);
        return Err(HistoryWriteError::Io(error));
    }
    let _ = fs::remove_file(backup);
    Ok(())
}

fn write_staged(path: &Path, contents: &[u8]) -> Result<(), HistoryWriteError> {
    let mut file = OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(path)
        .map_err(HistoryWriteError::Io)?;
    file.write_all(contents).map_err(HistoryWriteError::Io)?;
    file.sync_all().map_err(HistoryWriteError::Io)
}

fn unique_temporary(root: &Path) -> PathBuf {
    let suffix = TEMP_COUNTER.fetch_add(1, Ordering::Relaxed);
    root.join(HISTORY_DIR)
        .join(format!("runs.{}.{}.jsonl.tmp", std::process::id(), suffix))
}

fn restore_backup(primary: &Path, backup: &Path) {
    if backup.exists() && !primary.exists() {
        let _ = fs::rename(backup, primary);
    }
}

struct LedgerLock {
    _file: File,
}

impl LedgerLock {
    fn acquire(root: &Path) -> Result<Self, HistoryWriteError> {
        let path = root.join(LOCK_FILE);
        let file = OpenOptions::new()
            .read(true)
            .write(true)
            .create(true)
            .truncate(false)
            .open(path)
            .map_err(HistoryWriteError::Io)?;
        let deadline = Instant::now() + Duration::from_secs(5);
        loop {
            match file.try_lock() {
                Ok(()) => return Ok(Self { _file: file }),
                Err(TryLockError::WouldBlock) if Instant::now() < deadline => {
                    std::thread::sleep(Duration::from_millis(1));
                }
                Err(TryLockError::WouldBlock) => return Err(HistoryWriteError::Busy),
                Err(TryLockError::Error(error)) => return Err(HistoryWriteError::Io(error)),
            }
        }
    }
}

#[cfg(test)]
mod tests;
