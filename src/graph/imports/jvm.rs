use std::collections::{BTreeMap, HashSet};

pub(super) fn extract_java_spans(content: &str) -> BTreeMap<String, (usize, usize)> {
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

pub(super) fn extract_java(content: &str) -> HashSet<String> {
    extract_java_spans(content).into_keys().collect()
}

pub(super) fn extract_kotlin_spans(content: &str) -> BTreeMap<String, (usize, usize)> {
    let mut result = BTreeMap::new();
    for (i, line) in content.lines().enumerate() {
        let trimmed = line.trim();
        if trimmed.starts_with("//") || trimmed.starts_with("/*") || trimmed.starts_with('*') {
            continue;
        }
        if let Some(rest) = trimmed.strip_prefix("import ") {
            let rest = rest.trim_end_matches(';').trim();
            let base = rest.split(" as ").next().unwrap_or(rest).trim();
            if !base.is_empty() && !base.ends_with('*') {
                result.entry(base.to_string()).or_insert((i + 1, i + 1));
            }
        }
    }
    result
}

pub(super) fn extract_kotlin(content: &str) -> HashSet<String> {
    extract_kotlin_spans(content).into_keys().collect()
}
