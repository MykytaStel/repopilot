use crate::analysis::parse::ParsedFile;
use crate::languages::import_support::extract_string_literal;
use std::collections::{BTreeMap, HashSet};

pub(super) fn eager(parsed: &ParsedFile) -> HashSet<String> {
    extract(parsed.content())
}

pub(super) fn spans(parsed: &ParsedFile) -> BTreeMap<String, (usize, usize)> {
    extract_spans(parsed.content())
}

fn extract_spans(content: &str) -> BTreeMap<String, (usize, usize)> {
    let mut result = BTreeMap::new();
    let mut in_import_block = false;

    for (i, line) in content.lines().enumerate() {
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
                result.entry(path.to_string()).or_insert((i + 1, i + 1));
            }
            continue;
        }

        if let Some(rest) = trimmed.strip_prefix("import ")
            && let Some(path) = extract_string_literal(rest.trim())
        {
            result.entry(path.to_string()).or_insert((i + 1, i + 1));
        }
    }

    result
}

fn extract(content: &str) -> HashSet<String> {
    extract_spans(content).into_keys().collect()
}

fn extract_go_import_path(line: &str) -> Option<&str> {
    let start = line.find('"')?;
    let rest = &line[start + 1..];
    let end = rest.find('"')?;
    Some(&rest[..end])
}
