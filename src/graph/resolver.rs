use std::collections::{HashMap, HashSet};
use std::path::{Component, Path, PathBuf};

/// Resolves a raw import string extracted from `from_file` to a concrete path
/// under `root`. Returns a path only when it exists in `known_files`.
pub fn resolve_import(
    raw_import: &str,
    from_file: &Path,
    root: &Path,
    known_files: &HashSet<PathBuf>,
) -> Option<PathBuf> {
    let ext = from_file.extension().and_then(|e| e.to_str()).unwrap_or("");

    match ext {
        "rs" => resolve_rust(raw_import, from_file, root, known_files),
        "ts" | "tsx" | "js" | "jsx" | "mjs" | "cjs" => {
            resolve_ts(raw_import, from_file, root, known_files)
        }
        "py" => resolve_python(raw_import, from_file, root, known_files),
        "go" => resolve_go(raw_import, root, known_files),
        "java" => resolve_jvm(raw_import, root, known_files, &["java"]),
        "kt" | "kts" => resolve_jvm(raw_import, root, known_files, &["kt", "java"]),
        _ => None,
    }
}

// ── Rust ─────────────────────────────────────────────────────────────────────

fn resolve_rust(
    raw: &str,
    from_file: &Path,
    root: &Path,
    known_files: &HashSet<PathBuf>,
) -> Option<PathBuf> {
    if let Some(name) = raw.strip_prefix("mod::") {
        let dir = rust_current_module_dir(from_file, root);
        return probe(
            &[
                dir.join(format!("{name}.rs")),
                dir.join(name).join("mod.rs"),
            ],
            known_files,
        );
    }

    if let Some(rest) = raw.strip_prefix("crate::") {
        let src_root = root.join("src");
        return resolve_rust_module_path(&src_root, rest, known_files);
    }

    if let Some(rest) = raw.strip_prefix("self::") {
        let module_dir = rust_current_module_dir(from_file, root);
        return resolve_rust_module_path(&module_dir, rest, known_files);
    }

    if raw.starts_with("super::") {
        let mut remaining = raw;
        let mut base = rust_current_module_dir(from_file, root);
        while let Some(rest) = remaining.strip_prefix("super::") {
            base = base.parent().unwrap_or(root).to_path_buf();
            remaining = rest;
        }
        return resolve_rust_module_path(&base, remaining, known_files);
    }

    None
}

fn resolve_rust_module_path(
    base_dir: &Path,
    module_path: &str,
    known_files: &HashSet<PathBuf>,
) -> Option<PathBuf> {
    let segments = module_path
        .split("::")
        .filter(|segment| !segment.is_empty() && *segment != "self")
        .collect::<Vec<_>>();

    for end in (1..=segments.len()).rev() {
        let base = segments[..end]
            .iter()
            .fold(base_dir.to_path_buf(), |path, segment| path.join(segment));
        if let Some(path) = probe_rust_module_file(&base, known_files) {
            return Some(path);
        }
    }

    None
}

fn probe_rust_module_file(base: &Path, known_files: &HashSet<PathBuf>) -> Option<PathBuf> {
    probe(
        &[base.with_extension("rs"), base.join("mod.rs")],
        known_files,
    )
}

fn rust_current_module_dir(from_file: &Path, root: &Path) -> PathBuf {
    let src_root = root.join("src");
    let file_name = from_file.file_name().and_then(|name| name.to_str());

    match file_name {
        Some("lib.rs" | "main.rs" | "mod.rs") => from_file
            .parent()
            .map(Path::to_path_buf)
            .unwrap_or(src_root),
        Some(_) => from_file.with_extension(""),
        None => src_root,
    }
}

// ── TypeScript / JavaScript ───────────────────────────────────────────────────

fn resolve_ts(
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

use std::sync::{OnceLock, RwLock};

static TSCONFIG_CACHE: OnceLock<RwLock<HashMap<PathBuf, Vec<TsAlias>>>> = OnceLock::new();
static GO_MODULE_CACHE: OnceLock<RwLock<HashMap<PathBuf, Option<String>>>> = OnceLock::new();

fn get_tsconfig_cache() -> &'static RwLock<HashMap<PathBuf, Vec<TsAlias>>> {
    TSCONFIG_CACHE.get_or_init(|| RwLock::new(HashMap::new()))
}

fn get_go_module_cache() -> &'static RwLock<HashMap<PathBuf, Option<String>>> {
    GO_MODULE_CACHE.get_or_init(|| RwLock::new(HashMap::new()))
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

// ── Python ────────────────────────────────────────────────────────────────────

fn resolve_python(
    raw: &str,
    from_file: &Path,
    root: &Path,
    known_files: &HashSet<PathBuf>,
) -> Option<PathBuf> {
    if raw.starts_with('.') {
        let dots = raw.chars().take_while(|c| *c == '.').count();
        let module = &raw[dots..];

        let mut dir = from_file.parent()?;
        for _ in 0..dots.saturating_sub(1) {
            dir = dir.parent().unwrap_or(dir);
        }

        return resolve_python_module_from_base(dir, module, known_files);
    }

    for base in [root.to_path_buf(), root.join("src")] {
        if let Some(path) = resolve_python_module_from_base(&base, raw, known_files) {
            return Some(path);
        }
    }

    None
}

fn resolve_python_module_from_base(
    base_dir: &Path,
    module: &str,
    known_files: &HashSet<PathBuf>,
) -> Option<PathBuf> {
    if module.is_empty() {
        return probe(&[base_dir.join("__init__.py")], known_files);
    }

    let segments = module
        .split('.')
        .filter(|segment| !segment.is_empty())
        .collect::<Vec<_>>();

    for end in (1..=segments.len()).rev() {
        let base = segments[..end]
            .iter()
            .fold(base_dir.to_path_buf(), |path, segment| path.join(segment));
        if let Some(path) = probe(
            &[base.with_extension("py"), base.join("__init__.py")],
            known_files,
        ) {
            return Some(path);
        }
    }

    None
}

// ── Go ────────────────────────────────────────────────────────────────────────

fn resolve_go(raw: &str, root: &Path, known_files: &HashSet<PathBuf>) -> Option<PathBuf> {
    if !raw.contains('/') {
        return None;
    }

    if let Some(module_name) = read_go_module_name(root)
        && let Some(rest) = strip_go_module_prefix(raw, &module_name)
    {
        let rel = rest.trim_start_matches('/');
        let base = if rel.is_empty() {
            root.to_path_buf()
        } else {
            root.join(rel)
        };
        if let Some(path) = probe_go_package(&normalize_path(&base), known_files) {
            return Some(path);
        }
    }

    let root_name = root
        .file_name()
        .and_then(|name| name.to_str())
        .unwrap_or("");
    if !root_name.is_empty()
        && let Some(rest) = strip_go_module_prefix(raw, root_name)
    {
        let rel = rest.trim_start_matches('/');
        let base = normalize_path(&root.join(rel));
        return probe_go_package(&base, known_files);
    }

    None
}

fn strip_go_module_prefix<'a>(raw: &'a str, module_name: &str) -> Option<&'a str> {
    raw.strip_prefix(module_name)
        .filter(|rest| rest.is_empty() || rest.starts_with('/'))
}

fn probe_go_package(base: &Path, known_files: &HashSet<PathBuf>) -> Option<PathBuf> {
    if let Some(path) = probe(&[base.with_extension("go")], known_files) {
        return Some(path);
    }

    let package_dir = normalize_path(base);
    known_files
        .iter()
        .filter(|path| path.extension().and_then(|ext| ext.to_str()) == Some("go"))
        .filter(|path| {
            path.file_name()
                .and_then(|name| name.to_str())
                .is_none_or(|name| !name.ends_with("_test.go"))
        })
        .filter(|path| path.parent() == Some(package_dir.as_path()))
        .min()
        .cloned()
}

fn read_go_module_name(root: &Path) -> Option<String> {
    let cache = get_go_module_cache();
    if let Some(cached) = cache.read().unwrap().get(root) {
        return cached.clone();
    }

    let module = std::fs::read_to_string(root.join("go.mod"))
        .ok()
        .and_then(|content| {
            content.lines().find_map(|line| {
                line.trim()
                    .strip_prefix("module ")
                    .map(|module| module.trim().to_string())
            })
        });
    cache
        .write()
        .unwrap()
        .insert(root.to_path_buf(), module.clone());
    module
}

// ── JVM (Java / Kotlin) ───────────────────────────────────────────────────────

/// Resolves a fully-qualified JVM class name (`com.example.Foo`) to a source
/// file. Tries the standard Maven/Gradle source-root layout first, then falls
/// back to bare `src/`.
fn resolve_jvm(
    raw: &str,
    root: &Path,
    known_files: &HashSet<PathBuf>,
    extensions: &[&str],
) -> Option<PathBuf> {
    let rel = raw.replace('.', "/");

    const SOURCE_ROOTS: &[&str] = &[
        "src/main/java",
        "src/main/kotlin",
        "src",
        "app/src/main/java",
        "app/src/main/kotlin",
    ];

    for src_root in SOURCE_ROOTS {
        let base = normalize_path(&root.join(src_root).join(&rel));
        let candidates: Vec<PathBuf> = extensions
            .iter()
            .map(|ext| base.with_extension(ext))
            .collect();
        if let Some(path) = probe(&candidates, known_files) {
            return Some(path);
        }
    }
    None
}

// ── Helpers ───────────────────────────────────────────────────────────────────

fn probe(candidates: &[PathBuf], known_files: &HashSet<PathBuf>) -> Option<PathBuf> {
    for candidate in candidates {
        let normalized = normalize_path(candidate);
        if known_files.contains(&normalized) {
            return Some(normalized);
        }
    }
    None
}

/// Resolves `.` and `..` components without touching the filesystem.
pub(crate) fn normalize_path(path: &Path) -> PathBuf {
    let mut out = PathBuf::new();
    for component in path.components() {
        match component {
            Component::ParentDir => {
                out.pop();
            }
            Component::CurDir => {}
            other => out.push(other),
        }
    }
    out
}
