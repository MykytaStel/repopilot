use crate::analysis::exports::extract_exports;
use crate::analysis::parse::ParsedFile;
use crate::analysis::symbols::JavaScriptSymbolFacts;
use crate::analysis::symbols::javascript::extract_javascript_symbol_facts;
use crate::graph::imports::{
    extract_deferred_imports_from, extract_guarded_optional_imports_from,
    extract_import_spans_from, extract_imports_from,
};
use crate::review::diff::DiffTarget;
use crate::review::signals::content::post_change_source_at_path;
use crate::scan::facts::FileFacts;
use crate::scan::parsed_cache::{ParsedFactsCache, ParsedFactsEntry, content_hash};
use std::path::Path;

// Cached relationship summaries may omit content hashes or outlive parsed
// entries. Current scan artifacts/context come from disk even for `--since`;
// rebuild from that same scope. Only pre-change exporter facts use selected refs.
pub(super) fn rebuild(
    root: &Path,
    path: &Path,
    file: &FileFacts,
    cache: &mut ParsedFactsCache,
) -> Option<JavaScriptSymbolFacts> {
    let source = post_change_source_at_path(root, path, DiffTarget::WorkingTree)?;
    let hash = content_hash(source.content());
    if let Some(facts) = cache.lookup_javascript_symbols(&hash, file.language.as_deref()) {
        return Some(facts);
    }
    let parsed = ParsedFile::new(source.content(), file.language.as_deref());
    let symbols = extract_javascript_symbol_facts(
        source.content(),
        file.language.as_deref(),
        parsed.tree()?,
    )?;
    cache.insert(ParsedFactsEntry {
        content_hash: hash,
        language: file.language.clone(),
        non_empty_lines: file.non_empty_lines,
        branch_count: file.branch_count,
        has_inline_tests: file.has_inline_tests,
        imports: extract_imports_from(&parsed, file.language.as_deref()),
        import_spans: extract_import_spans_from(&parsed, file.language.as_deref()),
        deferred_imports: extract_deferred_imports_from(&parsed, file.language.as_deref()),
        guarded_optional_imports: extract_guarded_optional_imports_from(
            &parsed,
            file.language.as_deref(),
        ),
        exports: extract_exports(source.content(), file.language.as_deref()),
        javascript_symbols: Some(symbols.clone()),
        syntax: (&parsed.syntax_summary()).into(),
    });
    Some(symbols)
}
