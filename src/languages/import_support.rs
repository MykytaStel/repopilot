//! Shared helpers for per-language import extractors.

pub(crate) fn extract_string_literal(s: &str) -> Option<&str> {
    if let Some(rest) = s.strip_prefix('"') {
        let end = rest.find('"')?;
        Some(&rest[..end])
    } else if let Some(rest) = s.strip_prefix('\'') {
        let end = rest.find('\'')?;
        Some(&rest[..end])
    } else if let Some(rest) = s.strip_prefix('`') {
        let end = rest.find('`')?;
        Some(&rest[..end])
    } else {
        None
    }
}
