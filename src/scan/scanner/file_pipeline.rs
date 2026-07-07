use crate::analysis::SyntaxSummary;
use crate::analysis::exports::extract_exports;
use crate::analysis::parse::ParsedFile;
use crate::audits::pipeline::FileAuditRegistration;
use crate::findings::types::Finding;
use crate::graph::imports::{extract_deferred_imports_from, extract_imports_from};
use crate::scan::config::ScanConfig;
use crate::scan::facts::FileFacts;
use rayon::prelude::*;

pub(super) struct FileAnalysisResult {
    pub(super) findings: Vec<Finding>,
    pub(super) imports: Vec<String>,
    pub(super) deferred_imports: Vec<String>,
    pub(super) exports: Vec<String>,
    pub(super) syntax: SyntaxSummary,
}

pub(super) fn analyze_file(
    file: &FileFacts,
    file_audits: &[FileAuditRegistration],
    config: &ScanConfig,
) -> FileAnalysisResult {
    let parsed = ParsedFile::for_facts(file);
    let (parsed_required, text_only): (Vec<_>, Vec<_>) = file_audits
        .iter()
        .partition(|registration| registration.requires_parsed_syntax());

    let mut findings = run_file_audits(file, config, &text_only);
    let (parsed_findings, (imports, deferred_imports, exports, syntax)) =
        if parsed_required.is_empty() {
            (
                Vec::new(),
                extract_parsed_artifacts(&parsed, file.language.as_deref()),
            )
        } else {
            rayon::join(
                || run_parsed_file_audits(file, &parsed, config, &parsed_required),
                || extract_parsed_artifacts(&parsed, file.language.as_deref()),
            )
        };

    findings.extend(parsed_findings);

    FileAnalysisResult {
        findings,
        imports,
        deferred_imports,
        exports,
        syntax,
    }
}

fn run_file_audits(
    file: &FileFacts,
    config: &ScanConfig,
    registrations: &[&FileAuditRegistration],
) -> Vec<Finding> {
    registrations
        .par_iter()
        .map(|registration| registration.run(file, config))
        .collect::<Vec<_>>()
        .into_iter()
        .flatten()
        .collect()
}

fn run_parsed_file_audits(
    file: &FileFacts,
    parsed: &ParsedFile,
    config: &ScanConfig,
    registrations: &[&FileAuditRegistration],
) -> Vec<Finding> {
    registrations
        .par_iter()
        .map(|registration| registration.run_parsed(file, parsed, config))
        .collect::<Vec<_>>()
        .into_iter()
        .flatten()
        .collect()
}

fn extract_parsed_artifacts(
    parsed: &ParsedFile,
    language: Option<&str>,
) -> (Vec<String>, Vec<String>, Vec<String>, SyntaxSummary) {
    let imports = extract_imports_from(parsed, language);
    let deferred_imports = extract_deferred_imports_from(parsed, language);
    let exports = extract_exports(parsed.content(), language);
    let syntax = parsed.syntax_summary();
    (imports, deferred_imports, exports, syntax)
}
