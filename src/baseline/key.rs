use crate::findings::types::Finding;
use std::env;
use std::path::Path;

pub fn stable_finding_key(finding: &Finding, root: &Path) -> String {
    let Some(evidence) = finding.evidence.first() else {
        return format!("{}:{}", finding.rule_id, normalize_key_part(&finding.title));
    };

    let path = normalized_relative_path(&evidence.path, root);
    let mut key = format!("{}:{path}", finding.rule_id);

    if evidence.line_start > 0 {
        key.push_str(&format!(":{}", evidence.line_start));
    }

    if let Some(line_end) = evidence.line_end
        && line_end != evidence.line_start
    {
        key.push_str(&format!("-{line_end}"));
    }

    key
}

pub fn normalized_relative_path(path: &Path, root: &Path) -> String {
    if let Ok(relative_path) = path.strip_prefix(root)
        && !relative_path.as_os_str().is_empty()
    {
        return clean_path_string(&relative_path.to_string_lossy());
    }

    let path_string = clean_path_string(&path.to_string_lossy());
    let root_string = clean_path_string(&root.to_string_lossy());

    if let Some(relative_path) = strip_clean_prefix(&path_string, &root_string) {
        return relative_path;
    }

    if path.is_absolute()
        && let Ok(current_dir) = env::current_dir()
    {
        let current_dir_string = clean_path_string(&current_dir.to_string_lossy());
        if let Some(relative_path) = strip_clean_prefix(&path_string, &current_dir_string) {
            return relative_path;
        }
    }

    if path.is_absolute() {
        return path_string
            .rsplit('/')
            .find(|component| !component.is_empty())
            .unwrap_or(".")
            .to_string();
    }

    path_string
}

fn strip_clean_prefix(path: &str, root: &str) -> Option<String> {
    if root.is_empty() || root == "." {
        return None;
    }

    if path == root {
        return Some(".".to_string());
    }

    path.strip_prefix(root)
        .and_then(|suffix| suffix.strip_prefix('/'))
        .map(clean_path_string)
}

fn clean_path_string(path: &str) -> String {
    let mut value = path.replace('\\', "/");

    while value.contains("//") {
        value = value.replace("//", "/");
    }

    while let Some(stripped) = value.strip_prefix("./") {
        value = stripped.to_string();
    }

    if value.is_empty() {
        ".".to_string()
    } else {
        value
    }
}

fn normalize_key_part(value: &str) -> String {
    value
        .trim()
        .to_lowercase()
        .chars()
        .map(|character| {
            if character.is_ascii_alphanumeric() || matches!(character, '.' | '-' | '_' | '/') {
                character
            } else {
                '-'
            }
        })
        .collect::<String>()
        .trim_matches('-')
        .to_string()
}
