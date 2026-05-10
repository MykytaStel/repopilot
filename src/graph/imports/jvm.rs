use std::collections::HashSet;

pub(super) fn extract_java(content: &str) -> HashSet<String> {
    let mut result = HashSet::new();
    for line in content.lines() {
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
                result.insert(rest.to_string());
            }
        }
    }
    result
}

pub(super) fn extract_kotlin(content: &str) -> HashSet<String> {
    let mut result = HashSet::new();
    for line in content.lines() {
        let trimmed = line.trim();
        if trimmed.starts_with("//") || trimmed.starts_with("/*") || trimmed.starts_with('*') {
            continue;
        }
        if let Some(rest) = trimmed.strip_prefix("import ") {
            let rest = rest.trim_end_matches(';').trim();
            let base = rest.split(" as ").next().unwrap_or(rest).trim();
            if !base.is_empty() && !base.ends_with('*') {
                result.insert(base.to_string());
            }
        }
    }
    result
}
