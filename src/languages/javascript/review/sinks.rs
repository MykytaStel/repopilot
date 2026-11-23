use crate::review::signals::taint::sinks::{Sink, SinkKind, callee_text, receiver_method};
use tree_sitter::Node;

pub(super) fn classify_sink<'a>(node: Node<'a>, content: &'a str) -> Option<Sink<'a>> {
    let (callee, args) = callee_text(node, content)?;
    let kind = sink_kind(callee)?;
    let mut sink = Sink::new(kind, args);
    if kind == SinkKind::Exec {
        sink.safe_exec_command = safe_exec_command(callee, args, content);
    }
    Some(sink)
}

fn sink_kind(callee: &str) -> Option<SinkKind> {
    if callee.ends_with(".query")
        || callee == "query"
        || callee.ends_with(".execute")
        || callee == "execute"
    {
        Some(SinkKind::Sql)
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
        Some(SinkKind::Exec)
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
        Some(SinkKind::FsWrite)
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
        Some(SinkKind::Network)
    } else {
        None
    }
}

fn safe_exec_command<'a>(callee: &str, args: Node<'a>, content: &str) -> Option<Node<'a>> {
    let vector_api = ["spawn", "spawnSync", "execFile", "execFileSync"]
        .iter()
        .any(|name| callee == *name || callee.ends_with(&format!(".{name}")));
    if !vector_api {
        return None;
    }
    let mut cursor = args.walk();
    let arguments = args.named_children(&mut cursor).collect::<Vec<_>>();
    let command = *arguments.first()?;
    if command.kind() != "string" {
        return None;
    }
    let second = arguments.get(1).copied();
    if second.is_some_and(|argument| {
        !matches!(argument.kind(), "array" | "object")
            && argument.utf8_text(content.as_bytes()).ok() != Some("undefined")
    }) {
        return None;
    }
    let options = match (second, arguments.get(2).copied()) {
        (Some(node), _) if node.kind() == "object" => Some(node),
        (_, third) => third,
    };
    if arguments.len() > 3
        || options.is_some_and(|options| !safe_exec_options(options, content))
        || is_nested_shell(command, second, content)
    {
        return None;
    }
    Some(command)
}

fn safe_exec_options(options: Node<'_>, content: &str) -> bool {
    if options.kind() != "object" {
        return false;
    }
    let mut shell_disabled = true;
    let mut cursor = options.walk();
    for property in options.named_children(&mut cursor) {
        let Some(disables_shell) = safe_exec_property(property, content) else {
            return false;
        };
        if let Some(disables_shell) = disables_shell {
            shell_disabled = disables_shell;
        }
    }
    shell_disabled
}

fn safe_exec_property(property: Node<'_>, content: &str) -> Option<Option<bool>> {
    if property.kind() != "pair" {
        return None;
    }
    let key = property.child_by_field_name("key")?;
    if key.kind() == "computed_property_name" {
        return None;
    }
    let key_text = key
        .utf8_text(content.as_bytes())
        .unwrap_or("")
        .trim_matches(['\'', '"']);
    if key_text != "shell" {
        return Some(None);
    }
    let shell_disabled = property
        .child_by_field_name("value")
        .and_then(|value| value.utf8_text(content.as_bytes()).ok())
        == Some("false");
    shell_disabled.then_some(Some(true))
}

fn is_nested_shell(command: Node<'_>, args: Option<Node<'_>>, content: &str) -> bool {
    let command = command.utf8_text(content.as_bytes()).unwrap_or("");
    if !crate::languages::shell::is_shell_executable(command) {
        return false;
    }
    let Some(args) = args.filter(|node| node.kind() == "array") else {
        return false;
    };
    let mut cursor = args.walk();
    crate::languages::shell::has_shell_command_flag(
        args.named_children(&mut cursor)
            .filter_map(|node| node.utf8_text(content.as_bytes()).ok()),
    )
}

fn callee_is_or_ends_with(callee: &str, names: &[&str]) -> bool {
    names
        .iter()
        .any(|name| callee == *name || callee.ends_with(&format!(".{name}")))
}
