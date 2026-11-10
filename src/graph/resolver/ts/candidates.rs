use super::{alias_candidate_bases, normalize_path, probe, tsconfig_paths};
use std::collections::HashSet;
use std::path::{Path, PathBuf};

const TS_EXTENSIONS: &[&str] = &["ts", "tsx", "d.ts", "js", "jsx"];
const TS_SOURCE_EXTENSIONS: &[&str] = &["ts", "tsx", "js", "jsx"];
const ASSET_EXTENSIONS: &[&str] = &[
    "css", "scss", "sass", "less", "styl", "json", "html", "htm", "svg", "png", "jpg", "jpeg",
    "gif", "webp", "avif", "ico", "bmp", "woff", "woff2", "ttf", "otf", "eot", "wasm", "vue",
    "svelte", "astro",
];

pub(super) fn probe_ts_extensions(
    base: &Path,
    raw: &str,
    known_files: &HashSet<PathBuf>,
) -> Option<PathBuf> {
    probe(&ts_candidates(base, raw), known_files)
}

pub(in crate::graph::resolver) fn definitive_local_candidates(
    raw: &str,
    from_file: &Path,
    root: &Path,
) -> Option<Vec<PathBuf>> {
    // A missing extensionless relative import may name generated code. The
    // graph still links a supported target when it exists.
    if raw.starts_with('.') && is_extensionless(raw) {
        return None;
    }

    let bases = if raw.starts_with('.') {
        vec![normalize_path(&from_file.parent()?.join(raw))]
    } else {
        alias_candidate_bases(raw, &tsconfig_paths(root))
    };
    if bases.is_empty() || is_asset_import(raw) {
        return None;
    }

    let mut candidates = bases
        .iter()
        .flat_map(|base| ts_candidates(base, raw))
        .collect::<Vec<_>>();
    let mut seen = HashSet::new();
    candidates.retain(|candidate| seen.insert(candidate.clone()));
    (!candidates.is_empty()).then_some(candidates)
}

fn ts_candidates(base: &Path, raw: &str) -> Vec<PathBuf> {
    if is_asset_import(raw) {
        return Vec::new();
    }

    let extension = Path::new(raw).extension().and_then(|value| value.to_str());
    if extension.is_some_and(|extension| matches!(extension, "mjs" | "cjs")) {
        return vec![base.to_path_buf()];
    }
    let has_code_extension =
        extension.is_some_and(|extension| matches!(extension, "ts" | "tsx" | "js" | "jsx"));
    let base_has_code_extension = base
        .extension()
        .and_then(|value| value.to_str())
        .is_some_and(|extension| matches!(extension, "ts" | "tsx" | "js" | "jsx"));
    if !has_code_extension && base_has_code_extension {
        return vec![base.to_path_buf()];
    }

    let extensions = if has_code_extension {
        TS_SOURCE_EXTENSIONS
    } else {
        TS_EXTENSIONS
    };
    let mut candidates = Vec::new();

    if raw.ends_with(".d.ts") {
        candidates.push(base.to_path_buf());
    }
    for extension in extensions {
        if has_code_extension {
            candidates.push(base.with_extension(extension));
        } else if let Some(name) = base.file_name().and_then(|value| value.to_str()) {
            candidates.push(base.with_file_name(format!("{name}.{extension}")));
        }
    }
    if !has_code_extension {
        for extension in extensions {
            candidates.push(base.join(format!("index.{extension}")));
        }
    }
    candidates
}

fn is_extensionless(raw: &str) -> bool {
    Path::new(raw).extension().is_none()
}

fn is_asset_import(raw: &str) -> bool {
    Path::new(raw)
        .extension()
        .and_then(|value| value.to_str())
        .is_some_and(|extension| ASSET_EXTENSIONS.contains(&extension))
}
