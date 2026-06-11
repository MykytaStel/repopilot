use crate::analysis::parse::ParsedFile;
use std::collections::BTreeMap;

use super::{go, jvm, python, rust, ts};

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

    let parsed = ParsedFile::new(content, language);
    let all_spans = match language {
        Some("Rust") => parsed
            .tree()
            .map(|tree| rust::extract_spans(tree, content))
            .unwrap_or_default(),
        Some("TypeScript")
        | Some("TypeScript React")
        | Some("JavaScript")
        | Some("JavaScript React") => parsed
            .tree()
            .map(|tree| ts::extract_spans(tree, content))
            .unwrap_or_default(),
        Some("Python") => parsed
            .tree()
            .map(|tree| python::extract_spans(tree, content))
            .unwrap_or_default(),
        Some("Go") => go::extract_spans(content),
        Some("Java") => jvm::extract_java_spans(content),
        Some("Kotlin") => jvm::extract_kotlin_spans(content),
        _ => BTreeMap::new(),
    };

    let mut result = BTreeMap::new();
    for specifier in specifiers {
        if let Some(&span) = all_spans.get(specifier) {
            result.insert(specifier.clone(), span);
        }
    }
    result
}
