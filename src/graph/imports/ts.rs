use crate::graph::imports::common::{extract_string_literal, is_relative};
use std::collections::HashSet;

pub(super) fn extract(content: &str) -> HashSet<String> {
    let mut result = HashSet::new();

    for line in content.lines() {
        let trimmed = line.trim();

        if trimmed.starts_with("//") || trimmed.starts_with('*') || trimmed.starts_with("/*") {
            continue;
        }

        if (trimmed.starts_with("import ") || trimmed.starts_with("export "))
            && trimmed.contains(" from ")
            && let Some(path) = extract_from_path(trimmed)
            && is_relative(path)
        {
            result.insert(path.to_string());
        }

        if trimmed.contains("require(")
            && let Some(path) = extract_require_path(trimmed)
            && is_relative(path)
        {
            result.insert(path.to_string());
        }
    }

    result
}

fn extract_from_path(line: &str) -> Option<&str> {
    let pos = line.rfind(" from ")?;
    let after = line[pos + 6..].trim();
    extract_string_literal(after)
}

fn extract_require_path(line: &str) -> Option<&str> {
    let pos = line.find("require(")?;
    let after = line[pos + 8..].trim();
    extract_string_literal(after)
}
