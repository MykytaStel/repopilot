use super::{ConfidenceTier, ReviewSignal, SignalFamily, build_signal};
use crate::analysis::api_contract::ApiContractChange as ContractChange;
use crate::findings::types::Confidence;
use crate::review::signals::api_contract::{RemovedExportSignal, SymbolKind};
use crate::rules::SignalSource;
use crate::scan::cache::stable_hash_hex;

pub(super) fn verification_step(kind: &str) -> Option<&'static str> {
    match kind {
        "behavioral.removed-export-still-imported" => Some(
            "Inspect the changed exporter and surviving caller import, then restore the export or update the caller before running the repository's type-check, build, or focused tests.",
        ),
        "behavioral.rust-public-function-arity-changed" => Some(
            "Inspect the changed Rust function signature and direct caller, then update the caller or restore the prior parameter count before running the Rust type-check or focused tests.",
        ),
        _ => None,
    }
}

pub(super) fn from_occurrence(occurrence: &RemovedExportSignal) -> ReviewSignal {
    let exporter = occurrence
        .exporter_path
        .to_string_lossy()
        .replace('\\', "/");
    let importer = occurrence
        .importer_path
        .to_string_lossy()
        .replace('\\', "/");
    let (kind, headline, detail, identity) = match occurrence.change {
        ContractChange::RemovedExport => removed(occurrence, &exporter, &importer),
        ContractChange::RustFunctionArity {
            before,
            after,
            call_arguments,
        } => arity(
            occurrence,
            &exporter,
            &importer,
            before,
            after,
            call_arguments,
        ),
    };
    let mut signal = build_signal(
        kind,
        SignalFamily::Behavioral,
        ConfidenceTier::DefinitelySensitive,
        Confidence::High,
        importer,
        Some(occurrence.line_start),
        headline,
        Some(detail),
        SignalSource::Ast,
    );
    signal.signal_id = stable_hash_hex(identity.as_bytes())[..16].to_string();
    signal.target_path = Some(exporter);
    signal.line_end = Some(occurrence.line_end);
    signal.evidence_lines = (occurrence.line_start..=occurrence.line_end).collect();
    signal
}

fn removed(
    occurrence: &RemovedExportSignal,
    exporter: &str,
    importer: &str,
) -> (&'static str, &'static str, String, String) {
    let symbol_kind = match occurrence.symbol_kind {
        SymbolKind::Value => "value",
        SymbolKind::Type => "type",
    };
    let rust = occurrence
        .importer_path
        .extension()
        .is_some_and(|ext| ext == "rs");
    let relation = if rust {
        "referenced directly as"
    } else {
        "imported as local binding"
    };
    let detail = format!(
        "Removed {symbol_kind} export '{}' from {exporter} remains {relation} '{}' via '{}'.",
        occurrence.exported_name, occurrence.local_name, occurrence.module_specifier,
    );
    let identity = format!(
        "behavioral.removed-export-still-imported\0{exporter}\0{}\0{importer}\0{}\0{}-{}\0{}-{}",
        occurrence.exported_name,
        occurrence.module_specifier,
        occurrence.line_start,
        occurrence.line_end,
        occurrence.byte_start,
        occurrence.byte_end,
    );
    (
        "behavioral.removed-export-still-imported",
        if rust {
            "removed public function is still referenced"
        } else {
            "removed export is still imported"
        },
        detail,
        identity,
    )
}

fn arity(
    occurrence: &RemovedExportSignal,
    exporter: &str,
    importer: &str,
    before: usize,
    after: usize,
    call_arguments: usize,
) -> (&'static str, &'static str, String, String) {
    let detail = format!(
        "Rust public function '{}' in {exporter} changed arity from {before} to {after}; direct call at {importer}:{} still passes {call_arguments} arguments.",
        occurrence.exported_name, occurrence.line_start,
    );
    let identity = format!(
        "behavioral.rust-public-function-arity-changed\0{exporter}\0{}\0{importer}\0{}\0{}-{}\0{}-{}\0{before}-{after}-{call_arguments}",
        occurrence.exported_name,
        occurrence.module_specifier,
        occurrence.line_start,
        occurrence.line_end,
        occurrence.byte_start,
        occurrence.byte_end,
    );
    (
        "behavioral.rust-public-function-arity-changed",
        "Rust public function arity changed while a direct caller still uses the old argument count",
        detail,
        identity,
    )
}
