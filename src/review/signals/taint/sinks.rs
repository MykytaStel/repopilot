//! Dangerous-sink classification for taint-lite.
//!
//! Mirrors the per-language sink patterns the behavioral "added X" detectors
//! already recognize (`src/review/signals/behavioral/{js,python,go}.rs`), but
//! returns the call's *argument node* so [`super::flow`] can inspect what flows
//! in. Kept text/callee-name based to match the behavioral detectors and stay
//! robust across the JS/TS, Python, and Go grammars.

use super::TaintLang;
use serde::Serialize;
use tree_sitter::Node;

/// The kind of dangerous operation a sink performs.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum SinkKind {
    /// A raw SQL query/exec (`.query`, `.execute`, `db.Exec`, …).
    Sql,
    /// A subprocess / shell execution (`child_process.exec`, `subprocess.run`, …).
    Exec,
    /// A filesystem write or path-controlled open (`fs.writeFile`, `open(.., 'w')`).
    FsWrite,
    /// An outbound network call (`fetch`, `requests.get`, `http.Get`, …).
    Network,
}

impl SinkKind {
    /// Human-readable label used in detail/headline text.
    pub fn label(self) -> &'static str {
        match self {
            Self::Sql => "raw SQL query",
            Self::Exec => "subprocess/exec",
            Self::FsWrite => "filesystem write",
            Self::Network => "network call",
        }
    }
}

/// A classified sink call: its kind and the AST node holding its arguments.
pub(super) struct Sink<'a> {
    pub kind: SinkKind,
    pub args: Node<'a>,
}

/// Classify `node` as a dangerous sink call, returning its kind and argument node.
pub(super) fn classify_sink<'a>(
    node: Node<'a>,
    content: &'a str,
    lang: TaintLang,
) -> Option<Sink<'a>> {
    match lang {
        TaintLang::Js => js_sink(node, content),
        TaintLang::Python => python_sink(node, content),
        TaintLang::Go => go_sink(node, content),
    }
}

fn callee_text<'a>(node: Node<'a>, content: &'a str) -> Option<(&'a str, Node<'a>)> {
    if node.kind() != "call_expression" && node.kind() != "call" {
        return None;
    }
    let callee = node.child_by_field_name("function")?;
    let args = node.child_by_field_name("arguments")?;
    let text = callee.utf8_text(content.as_bytes()).ok()?.trim();
    Some((text, args))
}

fn js_sink<'a>(node: Node<'a>, content: &'a str) -> Option<Sink<'a>> {
    let (callee, args) = callee_text(node, content)?;
    let kind = if callee.ends_with(".query")
        || callee == "query"
        || callee.ends_with(".execute")
        || callee == "execute"
    {
        SinkKind::Sql
    } else if callee_is_or_ends_with(
        callee,
        &[
            "exec",
            "spawn",
            "execFile",
            "fork",
            "execSync",
            "spawnSync",
            "execFileSync",
        ],
    ) {
        SinkKind::Exec
    } else if callee_is_or_ends_with(
        callee,
        &[
            "writeFile",
            "writeFileSync",
            "appendFile",
            "appendFileSync",
            "createWriteStream",
        ],
    ) {
        SinkKind::FsWrite
    } else if callee == "fetch"
        || callee == "axios"
        || receiver_method(
            callee,
            "axios",
            &[
                "request", "get", "post", "put", "patch", "delete", "head", "options",
            ],
        )
        || matches!(
            callee,
            "http.request" | "https.request" | "http.get" | "https.get"
        )
    {
        SinkKind::Network
    } else {
        return None;
    };
    Some(Sink { kind, args })
}

fn python_sink<'a>(node: Node<'a>, content: &'a str) -> Option<Sink<'a>> {
    let (callee, args) = callee_text(node, content)?;
    let kind = if callee.ends_with(".execute")
        || callee.ends_with(".executemany")
        || callee.ends_with(".query")
        || callee.ends_with(".raw")
    {
        SinkKind::Sql
    } else if receiver_method(
        callee,
        "subprocess",
        &[
            "run",
            "Popen",
            "call",
            "check_call",
            "check_output",
            "getoutput",
            "getstatusoutput",
        ],
    ) || callee == "os.system"
        || callee == "os.popen"
        || callee == "eval"
        || callee == "exec"
    {
        SinkKind::Exec
    } else if (callee == "open" && python_open_is_write(args, content))
        || callee.ends_with(".write_text")
        || callee.ends_with(".write_bytes")
    {
        SinkKind::FsWrite
    } else if receiver_method(
        callee,
        "requests",
        &[
            "request", "get", "post", "put", "patch", "delete", "head", "options",
        ],
    ) || receiver_method(
        callee,
        "httpx",
        &[
            "request", "get", "post", "put", "patch", "delete", "head", "options",
        ],
    ) || callee == "urllib.request.urlopen"
        || callee == "urlopen"
    {
        SinkKind::Network
    } else {
        return None;
    };
    Some(Sink { kind, args })
}

fn go_sink<'a>(node: Node<'a>, content: &'a str) -> Option<Sink<'a>> {
    let (callee, args) = callee_text(node, content)?;
    let kind = if callee.ends_with(".Query")
        || callee.ends_with(".QueryRow")
        || callee.ends_with(".QueryContext")
        || callee.ends_with(".Exec")
        || callee.ends_with(".ExecContext")
    {
        SinkKind::Sql
    } else if matches!(callee, "exec.Command" | "exec.CommandContext") {
        SinkKind::Exec
    } else if matches!(
        callee,
        "os.WriteFile" | "os.Create" | "os.CreateTemp" | "ioutil.WriteFile"
    ) || (callee == "os.OpenFile" && go_open_file_is_write(args, content))
    {
        SinkKind::FsWrite
    } else if matches!(
        callee,
        "http.Get"
            | "http.Post"
            | "http.PostForm"
            | "http.NewRequest"
            | "http.NewRequestWithContext"
            | "net.Dial"
            | "net.DialTimeout"
    ) || callee.ends_with(".Do")
        || callee.ends_with(".DialContext")
    {
        SinkKind::Network
    } else {
        return None;
    };
    Some(Sink { kind, args })
}

fn callee_is_or_ends_with(callee: &str, names: &[&str]) -> bool {
    names
        .iter()
        .any(|name| callee == *name || callee.ends_with(&format!(".{name}")))
}

fn receiver_method(callee: &str, receiver: &str, methods: &[&str]) -> bool {
    methods
        .iter()
        .any(|method| callee == format!("{receiver}.{method}"))
}

fn python_open_is_write(args: Node<'_>, content: &str) -> bool {
    let mut cursor = args.walk();
    for (index, arg) in args.named_children(&mut cursor).enumerate() {
        if index == 0 {
            continue;
        }

        let mode = if arg.kind() == "keyword_argument" {
            let Some(name) = arg.child_by_field_name("name") else {
                continue;
            };
            if name.utf8_text(content.as_bytes()).ok() != Some("mode") {
                continue;
            }
            arg.child_by_field_name("value")
        } else if index == 1 {
            Some(arg)
        } else {
            None
        };

        if mode.is_some_and(|mode| {
            let text = mode.utf8_text(content.as_bytes()).unwrap_or("");
            quoted_value(text).is_some_and(|value| {
                value
                    .chars()
                    .any(|character| matches!(character, 'w' | 'a' | 'x' | '+'))
            })
        }) {
            return true;
        }
    }
    false
}

fn go_open_file_is_write(args: Node<'_>, content: &str) -> bool {
    let mut cursor = args.walk();
    let Some(flags) = args.named_children(&mut cursor).nth(1) else {
        return false;
    };
    let text = flags.utf8_text(content.as_bytes()).unwrap_or("");
    [
        "os.O_WRONLY",
        "os.O_RDWR",
        "os.O_APPEND",
        "os.O_CREATE",
        "os.O_TRUNC",
    ]
    .iter()
    .any(|flag| text.contains(flag))
}

fn quoted_value(text: &str) -> Option<&str> {
    let text = text.trim();
    let quote_index = text.find(['\'', '"'])?;
    let quote = text.as_bytes()[quote_index];
    let value = &text[quote_index + 1..];
    let end = value.as_bytes().iter().position(|byte| *byte == quote)?;
    Some(&value[..end])
}
