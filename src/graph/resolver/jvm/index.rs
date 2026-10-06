//! Inventory-owned filename index; package boundaries and ambiguity remain JVM-owned.
use super::resolve_candidates;
use std::collections::{BTreeMap, HashSet};
use std::path::PathBuf;

pub(crate) struct JvmIndex<'a> {
    by_stem: BTreeMap<&'a str, Vec<&'a PathBuf>>,
}

impl<'a> JvmIndex<'a> {
    pub(crate) fn new(files: &'a HashSet<PathBuf>) -> Self {
        let mut by_stem: BTreeMap<&str, Vec<&PathBuf>> = BTreeMap::new();
        for path in files {
            if !path
                .extension()
                .and_then(|value| value.to_str())
                .is_some_and(|extension| matches!(extension, "kt" | "java"))
            {
                continue;
            }
            if let Some(stem) = path.file_stem().and_then(|value| value.to_str()) {
                by_stem.entry(stem).or_default().push(path);
            }
        }
        Self { by_stem }
    }

    pub(crate) fn resolve(&self, raw: &str, extensions: &[&str]) -> Option<PathBuf> {
        resolve_candidates(raw, extensions, |stem| {
            self.by_stem
                .get(stem)
                .into_iter()
                .flatten()
                .copied()
                .filter(|path| {
                    path.extension()
                        .and_then(|value| value.to_str())
                        .is_some_and(|extension| extensions.contains(&extension))
                })
                .collect()
        })
    }
}
