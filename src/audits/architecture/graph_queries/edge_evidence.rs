use std::collections::HashSet;
use std::path::{Path, PathBuf};

use crate::graph::imports::lines::import_line_spans;
use crate::graph::resolver::resolve_import;
use crate::scan::facts::FileFacts;

/// Helper to attach line-span evidence to a graph edge.
/// Scans the source file's `imports` to find the specifier that resolves
/// to the target file, then retrieves its exact line span.
/// Fallback without content or match -> line 1.
pub(crate) fn edge_evidence(
    source_facts: &FileFacts,
    target_relative: &Path,
    root: &Path,
    known_files: &HashSet<PathBuf>,
) -> (usize, Option<usize>) {
    let Some(content) = &source_facts.content else {
        return (1, None);
    };

    let target_absolute = root.join(target_relative);
    let target_normalized = crate::graph::resolver::normalize_path(&target_absolute);
    let source_absolute = root.join(&source_facts.path);
    let source_normalized = crate::graph::resolver::normalize_path(&source_absolute);

    // Find which raw import specifier(s) map to our target
    let mut matching_specifiers = Vec::new();

    for specifier in &source_facts.imports {
        if let Some(resolved) = resolve_import(specifier, &source_normalized, root, known_files)
            && resolved == target_normalized
        {
            matching_specifiers.push(specifier.clone());
        }
    }

    if matching_specifiers.is_empty() {
        return (1, None);
    }

    let spans = import_line_spans(
        content,
        source_facts.language.as_deref(),
        &matching_specifiers,
    );

    for spec in matching_specifiers {
        if let Some(&(start, end)) = spans.get(&spec) {
            return (start, if end > start { Some(end) } else { None });
        }
    }

    (1, None)
}
