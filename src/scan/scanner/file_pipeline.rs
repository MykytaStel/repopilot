use crate::analysis::SyntaxSummary;
use crate::analysis::exports::extract_exports;
use crate::analysis::parse::ParsedFile;
use crate::audits::pipeline::FileAuditRegistration;
use crate::findings::types::Finding;
use crate::graph::imports::{
    extract_deferred_imports_from, extract_import_spans_from, extract_imports_from,
};
use crate::scan::config::ScanConfig;
use crate::scan::facts::FileFacts;
use rayon::prelude::*;

pub(super) struct FileAnalysisResult {
    pub(super) findings: Vec<Finding>,
    pub(super) imports: Vec<String>,
    pub(super) import_spans: std::collections::BTreeMap<String, (usize, usize)>,
    pub(super) deferred_imports: Vec<String>,
    pub(super) exports: Vec<String>,
    pub(super) syntax: SyntaxSummary,
}

struct ParsedFileFacts {
    imports: Vec<String>,
    import_spans: std::collections::BTreeMap<String, (usize, usize)>,
    deferred_imports: Vec<String>,
    exports: Vec<String>,
    syntax: SyntaxSummary,
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
    let (parsed_findings, parsed_facts) = if parsed_required.is_empty() {
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
        imports: parsed_facts.imports,
        import_spans: parsed_facts.import_spans,
        deferred_imports: parsed_facts.deferred_imports,
        exports: parsed_facts.exports,
        syntax: parsed_facts.syntax,
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

fn extract_parsed_artifacts(parsed: &ParsedFile, language: Option<&str>) -> ParsedFileFacts {
    let imports = extract_imports_from(parsed, language);
    let import_spans = extract_import_spans_from(parsed, language);
    let deferred_imports = extract_deferred_imports_from(parsed, language);
    let exports = extract_exports(parsed.content(), language);
    let syntax = parsed.syntax_summary();
    ParsedFileFacts {
        imports,
        import_spans,
        deferred_imports,
        exports,
        syntax,
    }
}
