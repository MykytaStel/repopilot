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
            resolve_ts(raw_import, from_file, known_files)
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

fn resolve_ts(raw: &str, from_file: &Path, known_files: &HashSet<PathBuf>) -> Option<PathBuf> {
    if !raw.starts_with('.') && !raw.starts_with('/') {
        return None;
    }

    let dir = from_file.parent()?;
    let base = if raw.starts_with('/') {
        PathBuf::from(raw)
    } else {
        dir.join(raw)
    };
    let base = normalize_path(&base);

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
