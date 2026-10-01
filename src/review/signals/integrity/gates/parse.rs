//! Parsers that flatten configuration into comparable `section.key` paths:
//! JSON/YAML values, TOML tables, INI files, and JSON with comments.

use serde_json::Value as Json;
use std::collections::BTreeMap;

pub(super) fn number(text: &str) -> Option<f64> {
    text.trim()
        .trim_end_matches('%')
        .trim_matches('"')
        .parse()
        .ok()
}

pub(super) fn flatten_json(prefix: &str, value: &Json, out: &mut BTreeMap<String, String>) {
    match value {
        Json::Object(map) => {
            for (key, child) in map {
                let path = if prefix.is_empty() {
                    key.clone()
                } else {
                    format!("{prefix}.{key}")
                };
                flatten_json(&path, child, out);
            }
        }
        Json::Array(items) => {
            let joined: Vec<String> = items
                .iter()
                .map(|item| {
                    item.as_str()
                        .map_or_else(|| item.to_string(), str::to_string)
                })
                .collect();
            out.insert(prefix.to_string(), joined.join(","));
        }
        Json::String(text) => {
            out.insert(prefix.to_string(), text.clone());
        }
        other => {
            out.insert(prefix.to_string(), other.to_string());
        }
    }
}

pub(super) fn flatten_toml(prefix: &str, value: &toml::Value, out: &mut BTreeMap<String, String>) {
    match value {
        toml::Value::Table(table) => {
            for (key, child) in table {
                let path = if prefix.is_empty() {
                    key.clone()
                } else {
                    format!("{prefix}.{key}")
                };
                flatten_toml(&path, child, out);
            }
        }
        toml::Value::Array(items) => {
            let joined: Vec<String> = items
                .iter()
                .map(|item| {
                    item.as_str()
                        .map_or_else(|| item.to_string(), str::to_string)
                })
                .collect();
            out.insert(prefix.to_string(), joined.join(","));
        }
        toml::Value::String(text) => {
            out.insert(prefix.to_string(), text.clone());
        }
        other => {
            out.insert(prefix.to_string(), other.to_string());
        }
    }
}

/// `[section] key = value` pairs as `section.key`, continuation lines joined.
pub(super) fn parse_ini(content: &str) -> BTreeMap<String, String> {
    let mut out = BTreeMap::new();
    let (mut section, mut last_key) = (String::new(), None::<String>);
    for raw in content.lines() {
        if raw.trim().is_empty() || raw.trim_start().starts_with(['#', ';']) {
            continue;
        }
        if raw.starts_with([' ', '\t']) {
            if let Some(key) = &last_key {
                out.entry(key.clone()).and_modify(|value: &mut String| {
                    value.push(' ');
                    value.push_str(raw.trim());
                });
            }
            continue;
        }
        let line = raw.trim();
        if let Some(name) = line
            .strip_prefix('[')
            .and_then(|rest| rest.strip_suffix(']'))
        {
            section = name.trim().to_string();
            last_key = None;
        } else if let Some((key, value)) = line.split_once(['=', ':']) {
            let key = format!("{section}.{}", key.trim());
            out.insert(key.clone(), value.trim().to_string());
            last_key = Some(key);
        }
    }
    out
}

/// JSON with comments and trailing commas (tsconfig).
pub(super) fn parse_jsonc(content: &str) -> Option<Json> {
    let mut clean = String::with_capacity(content.len());
    let mut chars = content.chars().peekable();
    let mut in_string = false;
    while let Some(c) = chars.next() {
        if in_string {
            clean.push(c);
            if c == '\\' {
                if let Some(next) = chars.next() {
                    clean.push(next);
                }
            } else if c == '"' {
                in_string = false;
            }
            continue;
        }
        match (c, chars.peek()) {
            ('"', _) => {
                in_string = true;
                clean.push(c);
            }
            ('/', Some('/')) => {
                for next in chars.by_ref() {
                    if next == '\n' {
                        clean.push('\n');
                        break;
                    }
                }
            }
            ('/', Some('*')) => {
                chars.next();
                let mut previous = ' ';
                for next in chars.by_ref() {
                    if previous == '*' && next == '/' {
                        break;
                    }
                    previous = next;
                }
            }
            _ => clean.push(c),
        }
    }
    let without_trailing_commas = trailing_commas(&clean);
    serde_json::from_str(&without_trailing_commas).ok()
}

fn trailing_commas(json: &str) -> String {
    let chars: Vec<char> = json.chars().collect();
    let mut out = String::with_capacity(json.len());
    for (index, c) in chars.iter().enumerate() {
        if *c == ',' {
            let next = chars[index + 1..].iter().find(|c| !c.is_whitespace());
            if matches!(next, Some('}') | Some(']')) {
                continue;
            }
        }
        out.push(*c);
    }
    out
}
