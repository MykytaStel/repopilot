use crate::analysis::parse::ParsedFile;
use std::collections::{BTreeMap, HashSet};

pub(super) fn eager(parsed: &ParsedFile) -> HashSet<String> {
    extract(parsed)
}

pub(super) fn spans(parsed: &ParsedFile) -> BTreeMap<String, (usize, usize)> {
    extract_spans(parsed)
}

fn extract_spans(parsed: &ParsedFile) -> BTreeMap<String, (usize, usize)> {
    let content = parsed.content();
    let mut result = BTreeMap::new();
    for (i, line) in content.lines().enumerate() {
        let trimmed = line.trim();
        if trimmed.starts_with("//") || trimmed.starts_with('*') || trimmed.starts_with("/*") {
            continue;
        }
        if let Some(rest) = trimmed.strip_prefix("import ") {
            let rest = rest
                .trim_start_matches("static ")
                .trim_end_matches(';')
                .trim();
            if !rest.is_empty() && !rest.ends_with('*') {
                result.entry(rest.to_string()).or_insert((i + 1, i + 1));
            }
        }
    }
    for (reference, span) in super::super::jvm_references::same_package_references(parsed) {
        result.entry(reference).or_insert(span);
    }
    result
}

fn extract(parsed: &ParsedFile) -> HashSet<String> {
    extract_spans(parsed).into_keys().collect()
}
