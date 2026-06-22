use crate::findings::types::Severity;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum RustPanicPattern {
    Unwrap,
    UnwrapErr,
    Expect,
    ExpectErr,
    Panic,
    Todo,
    Unimplemented,
}

impl RustPanicPattern {
    pub(super) fn label(self) -> &'static str {
        match self {
            RustPanicPattern::Unwrap => "unwrap()",
            RustPanicPattern::UnwrapErr => "unwrap_err()",
            RustPanicPattern::Expect => "expect()",
            RustPanicPattern::ExpectErr => "expect_err()",
            RustPanicPattern::Panic => "panic!",
            RustPanicPattern::Todo => "todo!",
            RustPanicPattern::Unimplemented => "unimplemented!",
        }
    }

    pub(super) fn signal(self) -> &'static str {
        match self {
            RustPanicPattern::Unwrap => "rust.unwrap",
            RustPanicPattern::UnwrapErr => "rust.unwrap_err",
            RustPanicPattern::Expect => "rust.expect",
            RustPanicPattern::ExpectErr => "rust.expect_err",
            RustPanicPattern::Panic => "rust.panic",
            RustPanicPattern::Todo => "rust.todo",
            RustPanicPattern::Unimplemented => "rust.unimplemented",
        }
    }

    pub(super) fn base_severity(self) -> Severity {
        match self {
            RustPanicPattern::Todo | RustPanicPattern::Unimplemented => Severity::High,
            RustPanicPattern::Unwrap
            | RustPanicPattern::UnwrapErr
            | RustPanicPattern::Expect
            | RustPanicPattern::ExpectErr
            | RustPanicPattern::Panic => Severity::Medium,
        }
    }

    /// Precedence used to pick a single pattern when more than one risky site
    /// lands on the same source line, mirroring the order of [`detect_pattern`]
    /// so AST detection emits one finding per line just like the line scanner.
    pub(super) fn precedence(self) -> u8 {
        match self {
            RustPanicPattern::Todo => 7,
            RustPanicPattern::Unimplemented => 6,
            RustPanicPattern::Panic => 5,
            RustPanicPattern::UnwrapErr => 4,
            RustPanicPattern::Unwrap => 3,
            RustPanicPattern::ExpectErr => 2,
            RustPanicPattern::Expect => 1,
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

    if trimmed.contains(".unwrap_err()") {
        return Some(RustPanicPattern::UnwrapErr);
    }

    if trimmed.contains(".unwrap()") {
        return Some(RustPanicPattern::Unwrap);
    }

    if trimmed.contains(".expect_err(") {
        return Some(RustPanicPattern::ExpectErr);
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
        RustPanicPattern::Unwrap
            | RustPanicPattern::UnwrapErr
            | RustPanicPattern::Expect
            | RustPanicPattern::ExpectErr
            | RustPanicPattern::Panic
    ) {
        return false;
    }

    let lower = sanitized.to_lowercase();

    // A lock acquisition that unwraps (`Mutex::lock().unwrap()`,
    // `RwLock::write().unwrap()`) is a poisoned-lock panic — an in-process
    // invariant, not an external I/O or database failure. Without this guard a
    // field named `db`/`conn`/`pool` (`self.db.lock().unwrap()`) would match the
    // `db.`/`conn.`/`pool.` substrings below and be escalated to High.
    if lower.contains(".lock()") {
        return false;
    }

    // Serializing an owned, in-memory value to JSON/YAML/TOML
    // (`serde_json::to_string(&x)`, `to_value`, `to_vec`, `to_writer`, ...) is an
    // in-process, effectively-infallible operation, not the parsing of untrusted
    // external input that the `serde_json`/`json` signals below are meant to
    // catch. Without this guard a serialization unwrap is escalated to a visible
    // High the same way a genuine deserialization is — the dominant panic-risk
    // false positive on services that persist structured data. Deserialization
    // (`from_str`/`from_slice`/`from_reader`/`from_value`) is left to escalate.
    if is_serialize_call(&lower) && !is_deserialize_call(&lower) {
        return false;
    }

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

/// True when the (lowercased) line invokes a serde serialization free function
/// (`serde_json::to_string`, `serde_yaml::to_vec`, ...). Anchored on the `::to_`
/// form so an ordinary infallible method call (`value.to_string()`) is ignored.
fn is_serialize_call(lower: &str) -> bool {
    const SERIALIZE_CALLS: &[&str] = &[
        "::to_string(",
        "::to_string_pretty(",
        "::to_value(",
        "::to_vec(",
        "::to_vec_pretty(",
        "::to_writer(",
        "::to_writer_pretty(",
    ];
    SERIALIZE_CALLS.iter().any(|call| lower.contains(call))
}

/// True when the line parses external bytes back into a value, which genuinely
/// fails on malformed input and so remains an external-failure path.
fn is_deserialize_call(lower: &str) -> bool {
    const DESERIALIZE_CALLS: &[&str] = &["from_str", "from_slice", "from_reader", "from_value"];
    DESERIALIZE_CALLS.iter().any(|call| lower.contains(call))
}

/// True when `node` is an `unwrap`/`expect` chained directly onto a literal
/// pattern constructor — `Regex::new("literal")` or `Selector::parse("literal")`
/// — which only fails on a malformed compile-time pattern: a deterministic bug
/// caught on the first run, not a runtime panic risk from external input. The
/// escalation that made these **visible High** on scraper-shaped files is
/// context-driven, so it is not reached by the `is_external_failure_path` guards;
/// this structural check skips the candidate outright, and because it walks the
/// syntax tree it also handles the multi-line `Regex::new(\n    r"…",\n).unwrap()`
/// form that the text-based [`should_ignore_contextual_panic_pattern`] cannot see.
pub(super) fn is_infallible_literal_construction_unwrap(
    node: tree_sitter::Node<'_>,
    content: &str,
) -> bool {
    unwrap_or_expect_receiver(node, content)
        .is_some_and(|receiver| is_literal_infallible_constructor_call(receiver, content))
}

/// True when `node` is `"<string literal>".parse().unwrap()`/`.expect(...)`. A
/// `.parse()` on a string literal is *not* statically infallible — `"999"
/// .parse::<u8>()` panics on overflow — but the value is authored, so any failure
/// is a deterministic bug caught on the first run, not external-input risk. Such
/// a call is therefore downgraded to Low (hidden in default, kept in strict)
/// rather than escalated like a `.parse()` on a runtime value (`raw.parse()`).
pub(super) fn is_literal_parse_unwrap(node: tree_sitter::Node<'_>, content: &str) -> bool {
    unwrap_or_expect_receiver(node, content)
        .is_some_and(|receiver| is_literal_parse_call(receiver, content))
}

/// Returns the receiver of `node` when `node` is `<receiver>.unwrap()` or
/// `<receiver>.expect(...)`. `unwrap_err`/`expect_err` assert the *Err* arm and
/// are deliberately excluded.
fn unwrap_or_expect_receiver<'a>(
    node: tree_sitter::Node<'a>,
    content: &str,
) -> Option<tree_sitter::Node<'a>> {
    if node.kind() != "call_expression" {
        return None;
    }
    let function = node.child_by_field_name("function")?;
    if function.kind() != "field_expression" {
        return None;
    }
    let field = function.child_by_field_name("field")?;
    let method = field.utf8_text(content.as_bytes()).ok()?;
    if method != "unwrap" && method != "expect" {
        return None;
    }
    function.child_by_field_name("value")
}

/// True when `node` is `"<string literal>".parse()` — a `.parse()` invoked
/// directly on a string literal.
fn is_literal_parse_call(node: tree_sitter::Node<'_>, content: &str) -> bool {
    if node.kind() != "call_expression" {
        return false;
    }
    let Some(function) = node.child_by_field_name("function") else {
        return false;
    };
    if function.kind() != "field_expression" {
        return false;
    }
    let Some(field) = function.child_by_field_name("field") else {
        return false;
    };
    if field.utf8_text(content.as_bytes()) != Ok("parse") {
        return false;
    }
    let Some(receiver) = function.child_by_field_name("value") else {
        return false;
    };
    matches!(receiver.kind(), "string_literal" | "raw_string_literal")
}

/// True when `node` is `Regex::new(<string literal>)` or
/// `Selector::parse(<string literal>)` (path-qualified forms such as
/// `regex::Regex::new` are accepted). A non-literal first argument means the
/// pattern is built at runtime and can genuinely fail, so it is left as a risk.
fn is_literal_infallible_constructor_call(node: tree_sitter::Node<'_>, content: &str) -> bool {
    const INFALLIBLE_LITERAL_CTORS: &[&str] = &["Regex::new", "Selector::parse"];

    if node.kind() != "call_expression" {
        return false;
    }
    let Some(callee) = node.child_by_field_name("function") else {
        return false;
    };
    let Ok(callee_text) = callee.utf8_text(content.as_bytes()) else {
        return false;
    };
    let callee_text = callee_text.trim();
    let is_known_ctor = INFALLIBLE_LITERAL_CTORS
        .iter()
        .any(|ctor| callee_text == *ctor || callee_text.ends_with(&format!("::{ctor}")));
    if !is_known_ctor {
        return false;
    }

    let Some(arguments) = node.child_by_field_name("arguments") else {
        return false;
    };
    let mut cursor = arguments.walk();
    for child in arguments.children(&mut cursor) {
        match child.kind() {
            "(" | ")" | "," => continue,
            "string_literal" | "raw_string_literal" => return true,
            // The first real argument is not a literal: the pattern is dynamic.
            _ => return false,
        }
    }
    false
}

/// Text fallback for `"literal".parse().unwrap()` when it appears inside a macro
/// such as `vec![ "path:fg:magenta".parse().unwrap(), ... ]`. The macro body is
/// an unparsed token tree, so the structural
/// [`is_infallible_literal_construction_unwrap`] check cannot see the expression.
/// Matches a (trimmed) line that begins with a string literal immediately
/// followed by `.parse()`/`.parse::<…>()` and an `.unwrap()`/`.expect(…)`.
pub(super) fn is_literal_parse_unwrap_line(trimmed: &str) -> bool {
    // The string-literal segment may be `"…"`, `r"…"`, or `r#"…"#`; take the text
    // after the matching close quote. Intentionally narrow: only a line that
    // *begins* with the literal is matched, so a `.parse()` on a runtime value is
    // never swept in. (A `r#"…"#` whose body contains `"` is left to the AST path.)
    let after_quote = if let Some(rest) = trimmed.strip_prefix("r#\"") {
        rest.split_once("\"#").map(|(_, tail)| tail)
    } else if let Some(rest) = trimmed
        .strip_prefix("r\"")
        .or_else(|| trimmed.strip_prefix('"'))
    {
        rest.split_once('"').map(|(_, tail)| tail)
    } else {
        None
    };
    let Some(tail) = after_quote.map(str::trim_start) else {
        return false;
    };
    // Only the untyped `.parse()` (target type inferred — the default-table
    // idiom) is downgraded; an explicit `.parse::<T>()` is a deliberate typed
    // conversion left to escalate, matching the AST path which does not recognize
    // the turbofish form.
    tail.starts_with(".parse()") && (tail.contains(".unwrap") || tail.contains(".expect"))
}

pub(super) fn is_infallible_render_write_start(path: &std::path::Path, trimmed: &str) -> bool {
    is_report_renderer_path(path)
        && (trimmed.starts_with("writeln!(") || trimmed.starts_with("write!("))
}

pub(super) fn is_report_renderer_path(path: &std::path::Path) -> bool {
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

pub(super) fn is_structural_infallible_render_write_unwrap(
    node: tree_sitter::Node<'_>,
    content: &str,
) -> bool {
    if node.kind() != "call_expression" {
        return false;
    }
    let Some(function) = node.child_by_field_name("function") else {
        return false;
    };
    if function.kind() != "field_expression" {
        return false;
    }
    let Some(value) = function.child_by_field_name("value") else {
        return false;
    };
    if value.kind() != "macro_invocation" {
        return false;
    }
    let Some(macro_node) = value
        .child_by_field_name("macro")
        .or_else(|| value.child(0))
    else {
        return false;
    };
    let Ok(macro_name) = macro_node.utf8_text(content.as_bytes()) else {
        return false;
    };
    macro_name == "write" || macro_name == "writeln"
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
        RustPanicPattern::UnwrapErr
        | RustPanicPattern::ExpectErr
        | RustPanicPattern::Panic
        | RustPanicPattern::Todo
        | RustPanicPattern::Unimplemented => false,
    }
}

fn is_infallible_render_write_line(trimmed: &str) -> bool {
    trimmed.starts_with("writeln!(") || trimmed.starts_with("write!(")
}
