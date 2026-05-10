use crate::graph::imports::common::extract_string_literal;
use std::collections::HashSet;

pub(super) fn extract(content: &str) -> HashSet<String> {
    let mut result = HashSet::new();
    let mut in_import_block = false;

    for line in content.lines() {
        let trimmed = line.trim();

        if trimmed.starts_with("//") {
            continue;
        }

        if trimmed == "import (" {
            in_import_block = true;
            continue;
        }

        if in_import_block {
            if trimmed == ")" {
                in_import_block = false;
                continue;
            }
            if let Some(path) = extract_go_import_path(trimmed) {
                result.insert(path.to_string());
            }
            continue;
        }

        if let Some(rest) = trimmed.strip_prefix("import ")
            && let Some(path) = extract_string_literal(rest.trim())
        {
            result.insert(path.to_string());
        }
    }

    result
}

fn extract_go_import_path(line: &str) -> Option<&str> {
    let start = line.find('"')?;
    let rest = &line[start + 1..];
    let end = rest.find('"')?;
    Some(&rest[..end])
}
