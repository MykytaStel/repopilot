//! One lazy JVM index per immutable graph inventory, shared across import lookups.
use super::{jvm::JvmIndex, resolve_import};
use std::cell::OnceCell;
use std::collections::HashSet;
use std::path::{Path, PathBuf};

pub(crate) struct ResolverInventory<'a> {
    files: &'a HashSet<PathBuf>,
    jvm: OnceCell<JvmIndex<'a>>,
}

impl<'a> ResolverInventory<'a> {
    pub(crate) fn new(files: &'a HashSet<PathBuf>) -> Self {
        Self {
            files,
            jvm: OnceCell::new(),
        }
    }

    pub(crate) fn resolve(&self, raw: &str, source: &Path, root: &Path) -> Option<PathBuf> {
        let extensions: &[&str] = match source.extension().and_then(|value| value.to_str()) {
            Some("java") => &["java"],
            Some("kt" | "kts") => &["kt", "java"],
            _ => return resolve_import(raw, source, root, self.files),
        };
        self.jvm
            .get_or_init(|| JvmIndex::new(self.files))
            .resolve(raw, extensions)
    }
}
