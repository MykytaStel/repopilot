use std::collections::HashSet;

pub(super) fn extract(content: &str) -> HashSet<String> {
    let mut result = HashSet::new();

    for line in content.lines() {
        let trimmed = line.trim();

        if trimmed.starts_with('#') {
            continue;
        }

        if let Some(rest) = trimmed.strip_prefix("from ") {
            if let Some(import_pos) = rest.find(" import ") {
                let module = rest[..import_pos].trim();
                if !module.is_empty() {
                    result.insert(module.to_string());
                }
            }
            continue;
        }

        if let Some(rest) = trimmed.strip_prefix("import ") {
            for part in rest.split(',') {
                let module = part.split(" as ").next().unwrap_or(part).trim();
                if !module.is_empty() {
                    result.insert(module.to_string());
                }
            }
        }
    }

    result
}
