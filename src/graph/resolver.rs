use std::collections::HashSet;
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
        "py" => resolve_python(raw_import, from_file, known_files),
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
    // mod::child  →  <same-dir>/child.rs  or  <same-dir>/child/mod.rs
    if let Some(name) = raw.strip_prefix("mod::") {
        let dir = from_file.parent().unwrap_or(root);
        return probe(
            &[
                dir.join(format!("{name}.rs")),
                dir.join(name).join("mod.rs"),
            ],
            known_files,
        );
    }

    // crate::a::b  →  root/src/a/b.rs  or  root/src/a/b/mod.rs
    if let Some(rest) = raw.strip_prefix("crate::") {
        let rel = rest.replace("::", "/");
        let base = root.join("src").join(&rel);
        return probe(
            &[base.with_extension("rs"), base.join("mod.rs")],
            known_files,
        );
    }

    None
}

// ── TypeScript / JavaScript ───────────────────────────────────────────────────

fn resolve_ts(
    raw: &str,
    from_file: &Path,
    root: &Path,
    known_files: &HashSet<PathBuf>,
) -> Option<PathBuf> {
    // ── 1. Relative imports (./ or ../) ───────────────────────────────────────
    if raw.starts_with('.') {
        let dir = from_file.parent()?;
        let base = normalize_path(&dir.join(raw));
        return probe_ts_extensions(&base, known_files);
    }

    // ── 2. Absolute imports starting with / ──────────────────────────────────
    if raw.starts_with('/') {
        let base = normalize_path(Path::new(raw));
        if let Some(resolved) = probe_ts_extensions(&base, known_files) {
            return Some(resolved);
        }
    }

    // ── 3. tsconfig / jsconfig path alias resolution ─────────────────────────
    let aliases = tsconfig_paths(root);
    resolve_ts_alias(raw, root, &aliases, known_files)
}

/// Probe a base path with all TS/JS extensions and index file variants.
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

// ── tsconfig path aliases ─────────────────────────────────────────────────────

/// One entry from `compilerOptions.paths`: a prefix pattern → list of replacement roots.
#[derive(Clone)]
struct TsAlias {
    /// The alias prefix, e.g. `@/*` or `@app/*` or `@utils` (exact).
    prefix: String,
    /// Whether the alias ends with `/*` (wildcard) or is an exact match.
    wildcard: bool,
    /// Replacement roots (relative to tsconfig dir), e.g. `["src/*"]`.
    /// The trailing `/*` is stripped; we join with the wildcard tail ourselves.
    roots: Vec<PathBuf>,
}

use std::cell::RefCell;
use std::collections::HashMap;

thread_local! {
    /// Cache of parsed tsconfig alias maps keyed by project root, so repeated
    /// calls within one scan only hit the filesystem once per root.
    static TSCONFIG_CACHE: RefCell<HashMap<PathBuf, Vec<TsAlias>>> =
        RefCell::new(HashMap::new());
}

/// Read and parse `compilerOptions.paths` from `tsconfig.json` or `jsconfig.json`.
/// Returns an empty vec when neither file exists or neither has `paths`.
fn tsconfig_paths(root: &Path) -> Vec<TsAlias> {
    TSCONFIG_CACHE.with(|cell| {
        let mut cache = cell.borrow_mut();
        if let Some(cached) = cache.get(root) {
            return cached.clone();
        }
        let aliases = parse_tsconfig_paths(root);
        cache.insert(root.to_path_buf(), aliases.clone());
        aliases
    })
}

fn parse_tsconfig_paths(root: &Path) -> Vec<TsAlias> {
    // Try tsconfig.json first, then jsconfig.json.
    let content = ["tsconfig.json", "jsconfig.json"]
        .iter()
        .find_map(|name| std::fs::read_to_string(root.join(name)).ok());

    let Some(content) = content else {
        return Vec::new();
    };

    // tsconfig uses JSON with comments — strip line comments before parsing.
    let stripped = strip_json_line_comments(&content);
    let Ok(value) = serde_json::from_str::<serde_json::Value>(&stripped) else {
        return Vec::new();
    };

    let paths = value
        .get("compilerOptions")
        .and_then(|co| co.get("paths"))
        .and_then(|p| p.as_object());

    let base_url = value
        .get("compilerOptions")
        .and_then(|co| co.get("baseUrl"))
        .and_then(|u| u.as_str())
        .map(|u| root.join(u));

    let Some(paths) = paths else {
        // Even without explicit paths, baseUrl alone can resolve bare imports.
        return base_url
            .map(|base| {
                vec![TsAlias {
                    prefix: String::new(), // matches everything (last resort)
                    wildcard: true,
                    roots: vec![base],
                }]
            })
            .unwrap_or_default();
    };

    let tsconfig_dir = root; // paths are relative to tsconfig location (root)

    let mut aliases: Vec<TsAlias> = paths
        .iter()
        .filter_map(|(pattern, mappings)| {
            let array = mappings.as_array()?;
            let roots: Vec<PathBuf> = array
                .iter()
                .filter_map(|v| v.as_str())
                .map(|mapping| {
                    // Strip trailing /* from mapping (e.g. "src/*" → "src")
                    let mapping = mapping.strip_suffix("/*").unwrap_or(mapping);
                    tsconfig_dir.join(mapping)
                })
                .collect();

            if roots.is_empty() {
                return None;
            }

            let wildcard = pattern.ends_with("/*");
            let prefix = pattern.strip_suffix("/*").unwrap_or(pattern).to_string();

            Some(TsAlias {
                prefix,
                wildcard,
                roots,
            })
        })
        .collect();

    // Longer (more specific) prefixes should be tried first.
    aliases.sort_by(|a, b| b.prefix.len().cmp(&a.prefix.len()));
    aliases
}

/// Strip `// …` line comments from JSON-with-comments (tsconfig format).
fn strip_json_line_comments(input: &str) -> String {
    let mut out = String::with_capacity(input.len());
    let mut in_string = false;
    let mut chars = input.chars().peekable();
    while let Some(ch) = chars.next() {
        match ch {
            '"' => {
                in_string = !in_string;
                out.push(ch);
            }
            '/' if !in_string => {
                if chars.peek() == Some(&'/') {
                    // consume rest of line
                    for c in chars.by_ref() {
                        if c == '\n' {
                            out.push('\n');
                            break;
                        }
                    }
                } else {
                    out.push(ch);
                }
            }
            '\\' if in_string => {
                out.push(ch);
                if let Some(next) = chars.next() {
                    out.push(next);
                }
            }
            _ => out.push(ch),
        }
    }
    out
}

/// Attempt to resolve `raw` against the collected tsconfig aliases.
fn resolve_ts_alias(
    raw: &str,
    _root: &Path,
    aliases: &[TsAlias],
    known_files: &HashSet<PathBuf>,
) -> Option<PathBuf> {
    for alias in aliases {
        if alias.prefix.is_empty() {
            // baseUrl-only alias: try root/raw
            let base = normalize_path(&alias.roots[0].join(raw));
            if let Some(p) = probe_ts_extensions(&base, known_files) {
                return Some(p);
            }
            continue;
        }

        if alias.wildcard {
            // Pattern: `@/*` matches `@/foo/bar` → tail = `foo/bar`
            let match_prefix = format!("{}/", alias.prefix);
            let Some(tail) = raw.strip_prefix(&match_prefix) else {
                continue;
            };
            for root in &alias.roots {
                let base = normalize_path(&root.join(tail));
                if let Some(p) = probe_ts_extensions(&base, known_files) {
                    return Some(p);
                }
            }
        } else {
            // Exact alias: `@utils` → mapped directory/file
            if raw != alias.prefix {
                continue;
            }
            for root in &alias.roots {
                if let Some(p) = probe_ts_extensions(root, known_files) {
                    return Some(p);
                }
            }
        }
    }
    None
}

// ── Python ────────────────────────────────────────────────────────────────────

fn resolve_python(raw: &str, from_file: &Path, known_files: &HashSet<PathBuf>) -> Option<PathBuf> {
    if !raw.starts_with('.') {
        return None;
    }

    let dots = raw.chars().take_while(|c| *c == '.').count();
    let module = &raw[dots..];

    let mut dir = from_file.parent()?;
    // One dot = current package (no parent navigation).
    // Two dots = parent package (navigate up once), etc.
    for _ in 0..dots.saturating_sub(1) {
        dir = dir.parent().unwrap_or(dir);
    }

    let base = if module.is_empty() {
        dir.to_path_buf()
    } else {
        let rel = module.replace('.', "/");
        dir.join(rel)
    };
    let base = normalize_path(&base);

    probe(
        &[base.with_extension("py"), base.join("__init__.py")],
        known_files,
    )
}

// ── Go ────────────────────────────────────────────────────────────────────────

fn resolve_go(raw: &str, root: &Path, known_files: &HashSet<PathBuf>) -> Option<PathBuf> {
    // Skip single-segment paths (stdlib packages like "fmt", "os")
    if !raw.contains('/') {
        return None;
    }

    // Prefer go.mod module name so that `github.com/user/project/subpkg`
    // resolves correctly regardless of the local directory name.
    if let Some(module_name) = read_go_module_name(root) {
        if let Some(rest) = raw.strip_prefix(&module_name) {
            let rel = rest.trim_start_matches('/');
            if !rel.is_empty() {
                let base = normalize_path(&root.join(rel));
                if let Some(p) = probe(&[base.with_extension("go")], known_files) {
                    return Some(p);
                }
            }
        }
    }

    // Fallback: match against the root directory name for projects without go.mod.
    let root_name = root.file_name().and_then(|n| n.to_str()).unwrap_or("");
    if !root_name.is_empty() {
        if let Some(rest) = raw.strip_prefix(root_name) {
            let rel = rest.trim_start_matches('/');
            let base = normalize_path(&root.join(rel));
            return probe(&[base.with_extension("go")], known_files);
        }
    }

    None
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
        if let Some(p) = probe(&candidates, known_files) {
            return Some(p);
        }
    }
    None
}

fn read_go_module_name(root: &Path) -> Option<String> {
    let content = std::fs::read_to_string(root.join("go.mod")).ok()?;
    content.lines().find_map(|line| {
        line.trim()
            .strip_prefix("module ")
            .map(|m| m.trim().to_string())
    })
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
