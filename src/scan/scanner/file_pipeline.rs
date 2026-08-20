use crate::analysis::SyntaxSummary;
use crate::analysis::exports::extract_exports;
use crate::analysis::parse::ParsedFile;
use crate::analysis::symbols::JavaScriptSymbolFacts;
use crate::analysis::symbols::javascript::extract_javascript_symbol_facts;
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
    pub(super) javascript_symbols: Option<JavaScriptSymbolFacts>,
    pub(super) syntax: SyntaxSummary,
}

struct ParsedFileFacts {
    imports: Vec<String>,
    import_spans: std::collections::BTreeMap<String, (usize, usize)>,
    deferred_imports: Vec<String>,
    exports: Vec<String>,
    javascript_symbols: Option<JavaScriptSymbolFacts>,
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
        javascript_symbols: parsed_facts.javascript_symbols,
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
    let javascript_symbols = parsed
        .tree()
        .and_then(|tree| extract_javascript_symbol_facts(parsed.content(), language, tree));
    let syntax = parsed.syntax_summary();
    ParsedFileFacts {
        imports,
        import_spans,
        deferred_imports,
        exports,
        javascript_symbols,
        syntax,
    }
}

#[cfg(test)]
mod tests {
    use super::analyze_file;
    use crate::scan::config::ScanConfig;
    use crate::scan::facts::FileFacts;
    use std::path::PathBuf;

    #[test]
    fn source_analysis_retains_typed_javascript_symbols() {
        let file = FileFacts {
            path: PathBuf::from("src/user.ts"),
            language: Some("TypeScript".to_string()),
            content: Some(
                concat!(
                    "export type UserId = string;\n",
                    "import { loadUser as load } from \"./api.ts\";\n",
                )
                .to_string(),
            ),
            ..FileFacts::default()
        };

        let result = analyze_file(&file, &[], &ScanConfig::default());
        let symbols = result.javascript_symbols.expect("supported symbol facts");

        assert_eq!(symbols.exports[0].name, "UserId");
        assert_eq!(symbols.imports[0].local_name, "load");
        assert_eq!(symbols.imports[0].module_specifier, "./api.ts");
    }

    #[test]
    fn malformed_source_does_not_claim_typed_javascript_symbols() {
        let file = FileFacts {
            path: PathBuf::from("src/user.ts"),
            language: Some("TypeScript".to_string()),
            content: Some("export type UserId = ;".to_string()),
            ..FileFacts::default()
        };

        let result = analyze_file(&file, &[], &ScanConfig::default());

        assert!(result.javascript_symbols.is_none());
    }
}
