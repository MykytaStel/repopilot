use super::{OVERLAY_PATH, OverlayEntry, OverlayValidation, parse_overlay_content};
use crate::scan::types::ScanDiagnostic;
use std::io;
use std::path::Path;
#[cfg(test)]
use std::path::PathBuf;
use std::sync::OnceLock;
use std::sync::atomic::{AtomicBool, Ordering};

pub struct OverlayRules {
    validation: OverlayValidation,
    matched: Vec<AtomicBool>,
}

static OVERLAY: OnceLock<OverlayRules> = OnceLock::new();

impl OverlayRules {
    pub fn load(root: &Path) -> io::Result<Self> {
        let overlay_path = root.join(OVERLAY_PATH);
        if !overlay_path.is_file() {
            return Ok(Self {
                validation: OverlayValidation {
                    overlay_path,
                    exists: false,
                    ..OverlayValidation::default()
                },
                matched: Vec::new(),
            });
        }
        let content = std::fs::read_to_string(&overlay_path)?;
        let validation = parse_overlay_content(&content, overlay_path);
        let matched = validation
            .entries
            .iter()
            .map(|_| AtomicBool::new(false))
            .collect();
        Ok(Self {
            validation,
            matched,
        })
    }

    pub fn exists(&self) -> bool {
        self.validation.exists
    }

    pub fn entries(&self) -> &[OverlayEntry] {
        &self.validation.entries
    }

    pub fn diagnostics(&self) -> &[ScanDiagnostic] {
        &self.validation.diagnostics
    }

    pub fn overlay_path(&self) -> &std::path::Path {
        &self.validation.overlay_path
    }

    #[cfg(test)]
    pub fn from_entries_for_test(entries: Vec<OverlayEntry>) -> Self {
        let matched = entries.iter().map(|_| AtomicBool::new(false)).collect();
        Self {
            validation: OverlayValidation {
                overlay_path: PathBuf::from(OVERLAY_PATH),
                exists: true,
                entries,
                invalid_entries_count: 0,
                parse_error: None,
                diagnostics: Vec::new(),
            },
            matched,
        }
    }

    pub(crate) fn mark_matched(&self, index_in_entries: usize) {
        if let Some(flag) = self.matched.get(index_in_entries) {
            flag.store(true, Ordering::Relaxed);
        }
    }

    /// Entries whose index was never marked matched during the scan —
    /// candidates for pruning. Surfaced as a scan diagnostic by
    /// `commands::product_scan`.
    pub fn unmatched_entries(&self) -> Vec<&OverlayEntry> {
        self.validation
            .entries
            .iter()
            .enumerate()
            .filter(|(i, _)| !self.matched[*i].load(Ordering::Relaxed))
            .map(|(_, entry)| entry)
            .collect()
    }
}

/// Initializes the process-wide overlay for `root`. Must be called exactly
/// once, before any `decide()` calls happen for this scan/review invocation
/// — the CLI and MCP server both operate on exactly one repo root per
/// process, so a single `OnceLock` (mirroring `active_knowledge()`) is safe.
///
/// This ordering is not enforced at runtime: if [`active_overlay`] is ever
/// called first, `OnceLock::get_or_init` permanently pins the process to an
/// empty ruleset, and every subsequent `init_active_overlay(root)` call
/// silently becomes a no-op — no error, no diagnostic. Today that hazard is
/// controlled entirely by there being exactly one, early production call
/// site (`src/commands/product_scan.rs`, wired in PR-2's Task 2.3), which
/// calls this before any scan mode runs and therefore before `decide()` is
/// reachable. If a second call site is ever added, whoever adds it is
/// responsible for preserving that "init before any decide()" ordering.
pub fn init_active_overlay(root: &Path) -> &'static OverlayRules {
    OVERLAY.get_or_init(|| {
        OverlayRules::load(root).unwrap_or_else(|_| OverlayRules {
            validation: OverlayValidation::default(),
            matched: Vec::new(),
        })
    })
}

pub fn active_overlay() -> &'static OverlayRules {
    OVERLAY.get_or_init(|| OverlayRules {
        validation: OverlayValidation::default(),
        matched: Vec::new(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn init_active_overlay_reads_the_repo_root_file() {
        let dir = tempfile::tempdir().expect("tempdir");
        let repopilot_dir = dir.path().join(".repopilot");
        std::fs::create_dir_all(&repopilot_dir).expect("mkdir");
        std::fs::write(
            repopilot_dir.join("overlay.toml"),
            r#"
                [[overlay]]
                rule = "architecture.large-file"
                severity = "low"
            "#,
        )
        .expect("write overlay.toml");

        let rules = OverlayRules::load(dir.path()).expect("load overlay");
        assert!(rules.exists());
        assert_eq!(rules.entries().len(), 1);
    }

    #[test]
    fn missing_overlay_file_is_not_an_error() {
        let dir = tempfile::tempdir().expect("tempdir");
        let rules = OverlayRules::load(dir.path()).expect("load overlay");
        assert!(!rules.exists());
        assert!(rules.entries().is_empty());
    }
}
