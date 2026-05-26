use crate::findings::types::Finding;
use crate::scan::cache::stable_hash_hex;
use std::env;
use std::path::Path;

pub fn stable_finding_key(finding: &Finding, root: &Path) -> String {
    let Some(evidence) = finding.evidence.first() else {
        return format!(
            "{}:{}",
            finding.rule_id,
            identity_fingerprint(&[finding.title.as_str(), finding.description.as_str(),])
        );
    };

    let path = normalized_relative_path(&evidence.path, root);
    let identity = if evidence.snippet.trim().is_empty() {
        identity_fingerprint(&[finding.title.as_str(), finding.description.as_str()])
    } else {
        identity_fingerprint(&[finding.title.as_str(), evidence.snippet.as_str()])
    };

    format!("{}:{path}:{identity}", finding.rule_id)
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
    // Collapse backslashes and consecutive slashes in one pass, then strip leading ./
    let value = path.replace('\\', "/");
    let value = collapse_slashes(&value);
    let value = value.trim_start_matches("./");

    if value.is_empty() {
        ".".to_string()
    } else {
        value.to_string()
    }
}

fn collapse_slashes(s: &str) -> String {
    if !s.contains("//") {
        return s.to_string();
    }
    let mut result = String::with_capacity(s.len());
    let mut prev_slash = false;
    for ch in s.chars() {
        if ch == '/' {
            if !prev_slash {
                result.push(ch);
            }
            prev_slash = true;
        } else {
            result.push(ch);
            prev_slash = false;
        }
    }
    result
}

fn identity_fingerprint(parts: &[&str]) -> String {
    let identity = parts
        .iter()
        .map(|part| normalize_identity_part(part))
        .collect::<Vec<_>>()
        .join("\n");
    stable_hash_hex(identity.as_bytes())
        .chars()
        .take(16)
        .collect()
}

fn normalize_identity_part(value: &str) -> String {
    let without_string_values = mask_string_literal_values(value);
    let mut normalized = String::with_capacity(without_string_values.len());
    let mut previous_was_space = false;
    let mut previous_was_digit = false;

    for character in without_string_values.chars().flat_map(char::to_lowercase) {
        if character.is_ascii_digit() {
            if !previous_was_digit {
                normalized.push('#');
            }
            previous_was_digit = true;
            previous_was_space = false;
            continue;
        }

        previous_was_digit = false;
        if character.is_whitespace() {
            if !previous_was_space {
                normalized.push(' ');
            }
            previous_was_space = true;
            continue;
        }

        normalized.push(character);
        previous_was_space = false;
    }

    normalized.trim().to_string()
}

fn mask_string_literal_values(value: &str) -> String {
    let mut output = String::with_capacity(value.len());
    let mut in_string = false;
    let mut escaped = false;

    for character in value.chars() {
        if in_string {
            match character {
                '\\' if !escaped => {
                    escaped = true;
                }
                '"' if !escaped => {
                    output.push('"');
                    in_string = false;
                }
                _ => {
                    escaped = false;
                }
            }
            continue;
        }

        output.push(character);
        if character == '"' {
            output.push('#');
            in_string = true;
            escaped = false;
        }
    }

    output
}
