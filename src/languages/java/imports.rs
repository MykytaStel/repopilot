use crate::analysis::parse::ParsedFile;
use std::collections::{BTreeMap, HashSet};

pub(super) fn eager(parsed: &ParsedFile) -> HashSet<String> {
    extract(parsed.content())
}

pub(super) fn spans(parsed: &ParsedFile) -> BTreeMap<String, (usize, usize)> {
    extract_spans(parsed.content())
}

fn extract_spans(content: &str) -> BTreeMap<String, (usize, usize)> {
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
    result
}

fn extract(content: &str) -> HashSet<String> {
    extract_spans(content).into_keys().collect()
}
