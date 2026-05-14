pub(super) fn extract_string_literal(s: &str) -> Option<&str> {
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

pub(super) fn is_relative(path: &str) -> bool {
    path.starts_with("./") || path.starts_with("../")
}
