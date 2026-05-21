use crate::findings::types::Severity;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum RustPanicPattern {
    Unwrap,
    Expect,
    Panic,
    Todo,
    Unimplemented,
}

impl RustPanicPattern {
    pub(super) fn label(self) -> &'static str {
        match self {
            RustPanicPattern::Unwrap => "unwrap()",
            RustPanicPattern::Expect => "expect()",
            RustPanicPattern::Panic => "panic!",
            RustPanicPattern::Todo => "todo!",
            RustPanicPattern::Unimplemented => "unimplemented!",
        }
    }

    pub(super) fn signal(self) -> &'static str {
        match self {
            RustPanicPattern::Unwrap => "rust.unwrap",
            RustPanicPattern::Expect => "rust.expect",
            RustPanicPattern::Panic => "rust.panic",
            RustPanicPattern::Todo => "rust.todo",
            RustPanicPattern::Unimplemented => "rust.unimplemented",
        }
    }

    pub(super) fn base_severity(self) -> Severity {
        match self {
            RustPanicPattern::Todo | RustPanicPattern::Unimplemented => Severity::High,
            RustPanicPattern::Unwrap | RustPanicPattern::Expect | RustPanicPattern::Panic => {
                Severity::Medium
            }
        }
    }
}

pub(super) fn detect_pattern(trimmed: &str) -> Option<RustPanicPattern> {
    if trimmed.contains("todo!(") {
        return Some(RustPanicPattern::Todo);
    }

    if trimmed.contains("unimplemented!(") {
        return Some(RustPanicPattern::Unimplemented);
    }

    if trimmed.contains("panic!(") {
        return Some(RustPanicPattern::Panic);
    }

    if trimmed.contains(".unwrap()") {
        return Some(RustPanicPattern::Unwrap);
    }

    if trimmed.contains(".expect(") {
        return Some(RustPanicPattern::Expect);
    }

    None
}

pub(super) fn should_ignore_contextual_panic_pattern(
    pattern: RustPanicPattern,
    raw_trimmed: &str,
) -> bool {
    if pattern != RustPanicPattern::Expect {
        return false;
    }

    let lower = raw_trimmed.to_lowercase();
    (lower.contains("valid") && lower.contains("regex") && lower.contains(".expect("))
        || (lower.contains("mutex") && lower.contains("poison") && lower.contains(".expect("))
}

pub(super) fn is_external_failure_path(pattern: RustPanicPattern, sanitized: &str) -> bool {
    if !matches!(
        pattern,
        RustPanicPattern::Unwrap | RustPanicPattern::Expect | RustPanicPattern::Panic
    ) {
        return false;
    }

    let lower = sanitized.to_lowercase();
    const EXTERNAL_SIGNALS: &[&str] = &[
        ".parse(",
        ".parse::<",
        "from_str(",
        "from_slice(",
        "serde_json",
        "json",
        "env::var",
        "std::env",
        "var_os(",
        "fs::",
        "std::fs",
        "file::open",
        "read_to_string",
        ".read_",
        ".recv",
        ".send",
        "request",
        "reqwest",
        "hyper",
        "axum",
        "headers",
        "query",
        "params",
        "body",
        "sqlx",
        "diesel",
        "postgres",
        "mysql",
        "redis",
        "database",
        "db.",
        "pool.",
        "conn.",
        "tcpstream",
        "udp",
        "socket",
    ];

    EXTERNAL_SIGNALS.iter().any(|signal| lower.contains(signal))
}

pub(super) fn is_infallible_render_write_start(path: &std::path::Path, trimmed: &str) -> bool {
    is_report_renderer_path(path)
        && (trimmed.starts_with("writeln!(") || trimmed.starts_with("write!("))
}

fn is_report_renderer_path(path: &std::path::Path) -> bool {
    let mut previous = None;
    for component in path
        .components()
        .filter_map(|component| component.as_os_str().to_str())
    {
        if previous == Some("src") && component == "output" {
            return true;
        }
        previous = Some(component);
    }
    false
}

pub(super) fn is_infallible_render_write_result_unwrap(
    pattern: RustPanicPattern,
    trimmed: &str,
) -> bool {
    match pattern {
        RustPanicPattern::Unwrap => {
            trimmed == ".unwrap();"
                || (is_infallible_render_write_line(trimmed) && trimmed.ends_with(".unwrap();"))
        }
        RustPanicPattern::Expect => {
            trimmed.starts_with(".expect(")
                || (is_infallible_render_write_line(trimmed) && trimmed.contains(").expect("))
        }
        RustPanicPattern::Panic | RustPanicPattern::Todo | RustPanicPattern::Unimplemented => false,
    }
}

fn is_infallible_render_write_line(trimmed: &str) -> bool {
    trimmed.starts_with("writeln!(") || trimmed.starts_with("write!(")
}
