use crate::analysis::parse::ParsedFile;
use crate::languages::imports_for_label;
use std::collections::BTreeMap;

/// Returns line spans (1-indexed) for the requested imported specifiers.
/// First occurrence wins. If a requested specifier isn't found, it is omitted.
pub fn import_line_spans(
    content: &str,
    language: Option<&str>,
    specifiers: &[String],
) -> BTreeMap<String, (usize, usize)> {
    if specifiers.is_empty() {
        return BTreeMap::new();
    }
    let Some(extractor) = language.and_then(imports_for_label) else {
        return BTreeMap::new();
    };

    let parsed = ParsedFile::new(content, language);
    let all_spans = (extractor.spans)(&parsed);

    let mut result = BTreeMap::new();
    for specifier in specifiers {
        if let Some(&span) = all_spans.get(specifier) {
            result.insert(specifier.clone(), span);
        }
    }
    result
}
