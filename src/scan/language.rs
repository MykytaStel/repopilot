use std::path::Path;

pub fn detect_language(path: &Path) -> Option<&'static str> {
    crate::knowledge::language::detect_language_for_path(path)
}
