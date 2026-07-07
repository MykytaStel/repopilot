use std::collections::BTreeSet;

pub(crate) fn extract_exports(content: &str, language: Option<&str>) -> Vec<String> {
    let exports = match language {
        Some("Rust") => rust_exports(content),
        Some("TypeScript")
        | Some("TypeScript React")
        | Some("JavaScript")
        | Some("JavaScript React") => javascript_exports(content),
        Some("Python") => python_exports(content),
        Some("Go") => go_exports(content),
        _ => BTreeSet::new(),
    };

    exports.into_iter().collect()
}

fn rust_exports(content: &str) -> BTreeSet<String> {
    let mut exports = BTreeSet::new();
    const PREFIXES: &[&str] = &[
        "pub async fn ",
        "pub fn ",
        "pub struct ",
        "pub enum ",
        "pub trait ",
        "pub type ",
        "pub const ",
        "pub static ",
        "pub mod ",
        "pub use ",
    ];

    for line in content.lines() {
        let line = line.trim_start();
        for prefix in PREFIXES {
            if let Some(rest) = line.strip_prefix(prefix) {
                if *prefix == "pub use " {
                    let clean = rest.trim_end_matches(|c: char| c == ';' || c.is_whitespace());
                    let target = if let Some(as_idx) = clean.rfind(" as ") {
                        &clean[as_idx + 4..]
                    } else {
                        clean.split("::").last().unwrap_or(clean)
                    };
                    if let Some(name) = first_identifier(target) {
                        exports.insert(name.to_string());
                    }
                } else {
                    if let Some(name) = first_identifier(rest) {
                        exports.insert(name.to_string());
                    }
                }
                break;
            }
        }
    }

    exports
}

fn javascript_exports(content: &str) -> BTreeSet<String> {
    let mut exports = BTreeSet::new();

    for line in content.lines() {
        let line = line.trim_start();
        let Some(rest) = line.strip_prefix("export ") else {
            continue;
        };

        if rest.starts_with("default ") {
            exports.insert("default".to_string());
            continue;
        }

        for prefix in [
            "async function ",
            "function ",
            "class ",
            "const ",
            "let ",
            "var ",
            "type ",
            "interface ",
            "enum ",
            "namespace ",
        ] {
            if let Some(value) = rest.strip_prefix(prefix) {
                if let Some(name) = first_identifier(value) {
                    exports.insert(name.to_string());
                }
                continue;
            }
        }

        if let Some(open) = rest.find('{')
            && let Some(close) = rest[open + 1..].find('}')
        {
            let list = &rest[open + 1..open + 1 + close];
            for item in list.split(',') {
                let item = item.trim().trim_start_matches("type ").trim();
                if item.is_empty() {
                    continue;
                }
                let name = item
                    .split_whitespace()
                    .collect::<Vec<_>>()
                    .windows(2)
                    .find_map(|parts| (parts[0] == "as").then_some(parts[1]))
                    .or_else(|| item.split_whitespace().next());
                if let Some(name) = name.and_then(first_identifier) {
                    exports.insert(name.to_string());
                }
            }
        }
    }

    exports
}

fn python_exports(content: &str) -> BTreeSet<String> {
    let mut exports = BTreeSet::new();

    for line in content.lines() {
        let line = line.trim();
        let Some(rest) = line.strip_prefix("__all__") else {
            continue;
        };
        let Some((_, value)) = rest.split_once('=') else {
            continue;
        };

        for item in value.split(',') {
            let name = item
                .trim()
                .trim_matches(|ch| matches!(ch, '[' | ']' | '(' | ')' | '"' | '\''));
            if is_identifier(name) {
                exports.insert(name.to_string());
            }
        }
    }

    exports
}

fn go_exports(content: &str) -> BTreeSet<String> {
    let mut exports = BTreeSet::new();

    for line in content.lines() {
        let line = line.trim_start();
        for prefix in ["func ", "type ", "var ", "const "] {
            let Some(rest) = line.strip_prefix(prefix) else {
                continue;
            };
            let rest = rest.trim_start_matches('(').trim_start();
            if let Some(name) = first_identifier(rest)
                && name.chars().next().is_some_and(char::is_uppercase)
            {
                exports.insert(name.to_string());
            }
        }
    }

    exports
}

fn first_identifier(value: &str) -> Option<&str> {
    let value = value.trim_start_matches(|ch: char| !ch.is_ascii_alphabetic() && ch != '_');
    let end = value
        .find(|ch: char| !ch.is_ascii_alphanumeric() && ch != '_')
        .unwrap_or(value.len());
    let identifier = &value[..end];
    is_identifier(identifier).then_some(identifier)
}

fn is_identifier(value: &str) -> bool {
    let mut chars = value.chars();
    chars
        .next()
        .is_some_and(|ch| ch.is_ascii_alphabetic() || ch == '_')
        && chars.all(|ch| ch.is_ascii_alphanumeric() || ch == '_')
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn extracts_rust_public_items_deterministically() {
        let source =
            "pub struct User;\npub fn load() {}\nfn hidden() {}\npub use crate::api::Client;\n";
        assert_eq!(
            extract_exports(source, Some("Rust")),
            vec!["Client", "User", "load"]
        );
    }

    #[test]
    fn extracts_javascript_named_and_default_exports() {
        let source = "export const value = 1;\nexport { thing as renamed, type Shape };\nexport default function App() {}\n";
        assert_eq!(
            extract_exports(source, Some("TypeScript")),
            vec!["Shape", "default", "renamed", "value"]
        );
    }

    #[test]
    fn unsupported_language_has_no_guessed_exports() {
        assert!(extract_exports("class Main {}", Some("Ruby")).is_empty());
    }
}
