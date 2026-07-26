use super::*;
use crate::config::model::HistorySection;
use crate::history::model::{AnalysisScope, ComparisonIdentity};
use std::sync::{Arc, Barrier};

#[test]
fn history_is_disabled_by_default() {
    assert!(!HistorySection::default().enabled);
}

#[test]
fn append_read_and_count_pruning_preserve_newest_receipts() {
    let temp = tempfile::tempdir().unwrap();
    let store = HistoryStore::new(
        temp.path(),
        HistoryLimits {
            max_runs: 2,
            max_bytes: DEFAULT_MAX_BYTES,
        },
    );
    for revision in 1..=3 {
        store.record(&receipt(revision)).unwrap();
    }
    let loaded = store.load();
    assert_eq!(loaded.receipts.len(), 2);
    assert_eq!(loaded.receipts[0].revision, "2");
    assert_eq!(loaded.receipts[1].revision, "3");
}

#[test]
fn truncated_tail_is_reported_without_losing_valid_receipts() {
    let temp = tempfile::tempdir().unwrap();
    let store = HistoryStore::new(temp.path(), HistoryLimits::default());
    store.record(&receipt(1)).unwrap();
    let mut file = OpenOptions::new()
        .append(true)
        .open(history_file_path(temp.path()))
        .unwrap();
    write!(file, "{{\"schema_version\":").unwrap();
    let loaded = store.load();
    assert_eq!(loaded.receipts.len(), 1);
    assert!(loaded.diagnostics.iter().any(|item| {
        item.kind == HistoryDiagnosticKind::TruncatedRecord && item.line == Some(2)
    }));
}

#[test]
fn retention_obeys_byte_limit() {
    let temp = tempfile::tempdir().unwrap();
    let line_size = serde_json::to_vec(&receipt(1)).unwrap().len() as u64 + 1;
    let store = HistoryStore::new(
        temp.path(),
        HistoryLimits {
            max_runs: 10,
            max_bytes: line_size * 2,
        },
    );
    for revision in 1..=3 {
        store.record(&receipt(revision)).unwrap();
    }
    assert_eq!(store.load().receipts.len(), 2);
    assert!(fs::metadata(history_file_path(temp.path())).unwrap().len() <= line_size * 2);
}

#[test]
fn concurrent_writers_preserve_every_complete_receipt() {
    let temp = tempfile::tempdir().unwrap();
    let store = Arc::new(HistoryStore::new(
        temp.path(),
        HistoryLimits {
            max_runs: 16,
            max_bytes: DEFAULT_MAX_BYTES,
        },
    ));
    let barrier = Arc::new(Barrier::new(8));
    let handles = (1..=8)
        .map(|revision| {
            let store = Arc::clone(&store);
            let barrier = Arc::clone(&barrier);
            std::thread::spawn(move || {
                barrier.wait();
                store.record(&receipt(revision))
            })
        })
        .collect::<Vec<_>>();

    for handle in handles {
        handle.join().unwrap().unwrap();
    }
    let loaded = store.load();
    assert!(loaded.diagnostics.is_empty());
    assert_eq!(loaded.receipts.len(), 8);
}

#[test]
fn newest_compatible_skips_a_newer_incompatible_receipt() {
    let temp = tempfile::tempdir().unwrap();
    let store = HistoryStore::new(temp.path(), HistoryLimits::default());
    let full = receipt(1);
    store.record(&full).unwrap();
    let mut changed = receipt(2);
    changed.comparison.scope = AnalysisScope::Changed;
    store.record(&changed).unwrap();

    let compatible = store.newest_compatible(&full.comparison).unwrap();

    assert_eq!(compatible.revision, "1");
}

#[test]
fn abandoned_lock_file_does_not_block_future_writes() {
    let temp = tempfile::tempdir().unwrap();
    let lock = temp.path().join(LOCK_FILE);
    fs::create_dir_all(lock.parent().unwrap()).unwrap();
    fs::write(lock, "abandoned").unwrap();

    HistoryStore::new(temp.path(), HistoryLimits::default())
        .record(&receipt(1))
        .unwrap();
}

fn receipt(revision: usize) -> RunReceipt {
    RunReceipt::new(
        ComparisonIdentity {
            workspace: "/repo".to_string(),
            analysis_target: ".".to_string(),
            scope: AnalysisScope::Full,
            base_revision: None,
            head_revision: Some(revision.to_string()),
            profile: "default".to_string(),
            config_fingerprint: "config".to_string(),
            selection_fingerprint: "selection".to_string(),
            overlay_fingerprint: "overlay".to_string(),
            analysis_schema: "scan-report-v0.23".to_string(),
        },
        format!("2026-07-26T12:00:0{revision}Z"),
        revision.to_string(),
        Vec::new(),
    )
}
