//! TypeScript / JavaScript import resolution, including `tsconfig`/`jsconfig`
//! `paths` and `baseUrl` aliases (parsed once and cached per repo root).

use super::{normalize_path, probe};
use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};
use std::sync::{OnceLock, RwLock};

pub(super) fn resolve_ts(
    raw: &str,
    from_file: &Path,
    root: &Path,
    known_files: &HashSet<PathBuf>,
) -> Option<PathBuf> {
    if raw.starts_with('.') {
        let dir = from_file.parent()?;
        let base = normalize_path(&dir.join(raw));
        return probe_ts_extensions(&base, known_files);
    }

    if raw.starts_with('/') {
        let base = normalize_path(&root.join(raw.trim_start_matches('/')));
        if let Some(path) = probe_ts_extensions(&base, known_files) {
            return Some(path);
        }
    }

    let aliases = tsconfig_paths(root);
    resolve_ts_alias(raw, &aliases, known_files)
}

fn probe_ts_extensions(base: &Path, known_files: &HashSet<PathBuf>) -> Option<PathBuf> {
    const EXTS: &[&str] = &["ts", "tsx", "js", "jsx"];
    let mut candidates: Vec<PathBuf> = Vec::new();
    for ext in EXTS {
        candidates.push(base.with_extension(ext));
    }
    for ext in EXTS {
        candidates.push(base.join(format!("index.{ext}")));
    }
    probe(&candidates, known_files)
}

#[derive(Clone)]
struct TsAlias {
    prefix: String,
    wildcard: bool,
    roots: Vec<PathBuf>,
}

static TSCONFIG_CACHE: OnceLock<RwLock<HashMap<PathBuf, Vec<TsAlias>>>> = OnceLock::new();

fn get_tsconfig_cache() -> &'static RwLock<HashMap<PathBuf, Vec<TsAlias>>> {
    TSCONFIG_CACHE.get_or_init(|| RwLock::new(HashMap::new()))
}

fn tsconfig_paths(root: &Path) -> Vec<TsAlias> {
    let cache = get_tsconfig_cache();
    if let Some(cached) = cache.read().unwrap().get(root) {
        return cached.clone();
    }
    let aliases = parse_tsconfig_paths(root);
    cache
        .write()
        .unwrap()
        .insert(root.to_path_buf(), aliases.clone());
    aliases
}

fn parse_tsconfig_paths(root: &Path) -> Vec<TsAlias> {
    let content = ["tsconfig.json", "jsconfig.json"]
        .iter()
        .find_map(|name| std::fs::read_to_string(root.join(name)).ok());

    let Some(content) = content else {
        return Vec::new();
    };

    let stripped = strip_json_line_comments(&content);
    let Ok(value) = serde_json::from_str::<serde_json::Value>(&stripped) else {
        return Vec::new();
    };

    let base_url = value
        .get("compilerOptions")
        .and_then(|options| options.get("baseUrl"))
        .and_then(|base| base.as_str())
        .map(|base| root.join(base));

    let Some(paths) = value
        .get("compilerOptions")
        .and_then(|options| options.get("paths"))
        .and_then(|paths| paths.as_object())
    else {
        return base_url
            .map(|base| {
                vec![TsAlias {
                    prefix: String::new(),
                    wildcard: true,
                    roots: vec![base],
                }]
            })
            .unwrap_or_default();
    };

    let base_url = base_url.unwrap_or_else(|| root.to_path_buf());
    let mut aliases = paths
        .iter()
        .filter_map(|(pattern, mappings)| {
            let roots = mappings
                .as_array()?
                .iter()
                .filter_map(|mapping| mapping.as_str())
                .map(|mapping| base_url.join(mapping.strip_suffix("/*").unwrap_or(mapping)))
                .collect::<Vec<_>>();

            if roots.is_empty() {
                return None;
            }

            Some(TsAlias {
                prefix: pattern.strip_suffix("/*").unwrap_or(pattern).to_string(),
                wildcard: pattern.ends_with("/*"),
                roots,
            })
        })
        .collect::<Vec<_>>();

    aliases.sort_by_key(|right| std::cmp::Reverse(right.prefix.len()));
    aliases
}

fn strip_json_line_comments(input: &str) -> String {
    let mut out = String::with_capacity(input.len());
    let mut chars = input.chars().peekable();
    let mut in_string = false;
    let mut escaped = false;

    while let Some(ch) = chars.next() {
        if in_string {
            out.push(ch);
            if escaped {
                escaped = false;
            } else if ch == '\\' {
                escaped = true;
            } else if ch == '"' {
                in_string = false;
            }
            continue;
        }

        if ch == '"' {
            in_string = true;
            out.push(ch);
        } else if ch == '/' && chars.peek() == Some(&'/') {
            for next in chars.by_ref() {
                if next == '\n' {
                    out.push('\n');
                    break;
                }
            }
        } else {
            out.push(ch);
        }
    }

    out
}

fn resolve_ts_alias(
    raw: &str,
    aliases: &[TsAlias],
    known_files: &HashSet<PathBuf>,
) -> Option<PathBuf> {
    for alias in aliases {
        if alias.prefix.is_empty() {
            let base = normalize_path(&alias.roots[0].join(raw));
            if let Some(path) = probe_ts_extensions(&base, known_files) {
                return Some(path);
            }
            continue;
        }

        if alias.wildcard {
            let match_prefix = format!("{}/", alias.prefix);
            let Some(tail) = raw.strip_prefix(&match_prefix) else {
                continue;
            };

            for root in &alias.roots {
                let base = normalize_path(&root.join(tail));
                if let Some(path) = probe_ts_extensions(&base, known_files) {
                    return Some(path);
                }
            }
        } else if raw == alias.prefix {
            for root in &alias.roots {
                if let Some(path) = probe_ts_extensions(root, known_files) {
                    return Some(path);
                }
            }
        }
    }

    None
}
