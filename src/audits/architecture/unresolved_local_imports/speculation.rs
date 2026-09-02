//! Tells a Python import that names a module apart from one that only *might*.
//!
//! `from . import get_document_model` is recorded as two imports: the package
//! `.` and the derived candidate `.get_document_model`. The derived candidate
//! is a guess — `get_document_model` is just as likely a function defined in the
//! package's `__init__.py`. Only the explicit form `from .get_document_model
//! import x` proves a module was meant, and it never records the parent.
//!
//! So a derived candidate is exactly a candidate whose strict parent prefix the
//! same file also imports, which is what this module detects.

use crate::graph::UnresolvedImportEvidence;
use crate::scan::facts::FileFacts;

pub(super) fn is_python_package_member(
    unresolved: &UnresolvedImportEvidence,
    source_facts: Option<&FileFacts>,
) -> bool {
    if unresolved
        .source
        .extension()
        .and_then(|extension| extension.to_str())
        != Some("py")
    {
        return false;
    }
    let Some(parent) = parent_module(&unresolved.raw_import) else {
        return false;
    };
    source_facts.is_some_and(|facts| facts.imports.iter().any(|import| import == parent))
}

// Return the parent module while preserving leading relative dots. Bare `.` and
// `..` have no parent candidate.
fn parent_module(raw_import: &str) -> Option<&str> {
    let leading_dots = raw_import.len() - raw_import.trim_start_matches('.').len();
    let (dots, rest) = raw_import.split_at(leading_dots);
    if rest.is_empty() {
        return None;
    }
    match rest.rfind('.') {
        Some(index) => Some(&raw_import[..leading_dots + index]),
        // A single segment after the dots: the parent is the relative package
        // itself, and an absolute `name` has no local parent to import.
        None => (!dots.is_empty()).then_some(dots),
    }
}

#[cfg(test)]
mod tests {
    use super::parent_module;

    #[test]
    fn parent_module_keeps_relative_depth_and_stops_at_the_package() {
        // Catches a parent that loses relative depth, which would compare the
        // candidate against an import the file never made.
        assert_eq!(parent_module(".get_document_model"), Some("."));
        assert_eq!(parent_module("..get_document_model"), Some(".."));
        assert_eq!(parent_module("...views"), Some("..."));
        assert_eq!(parent_module(".settings.local"), Some(".settings"));
        assert_eq!(parent_module("wagtail.documents"), Some("wagtail"));
        assert_eq!(parent_module("."), None);
        assert_eq!(parent_module(".."), None);
        assert_eq!(parent_module("settings"), None);
    }
}
