//! `repopilot mcp` — a local Model Context Protocol server over stdio.
//!
//! The server is deliberately synchronous: it reads newline-delimited JSON-RPC
//! 2.0 messages from stdin, dispatches each to a handler, and writes the
//! response to stdout. The workload is one client and request/response,
//! CPU-bound work (a scan), so there is no need for an async runtime — keeping
//! the binary lean and the protocol surface small and auditable, which matches
//! RepoPilot's local-first promise (nothing is uploaded; no AI service is
//! called).

mod context;
mod explain_file;
mod jsonrpc;
mod review_change;
mod scan;

#[cfg(test)]
mod tests;

use crate::cli::McpOptions;
use jsonrpc::{METHOD_NOT_FOUND, Request, Response};
use serde_json::{Value, json};
use std::io::{BufRead, Write};

const SERVER_NAME: &str = "repopilot";
const SERVER_VERSION: &str = env!("CARGO_PKG_VERSION");
/// Protocol version used when the client does not request one.
const DEFAULT_PROTOCOL_VERSION: &str = "2024-11-05";

pub fn run(_options: McpOptions) -> Result<(), Box<dyn std::error::Error>> {
    let stdin = std::io::stdin();
    let stdout = std::io::stdout();
    serve(stdin.lock(), stdout.lock())?;
    Ok(())
}

/// Drives the JSON-RPC loop until the input stream closes. Generic over the
/// reader and writer so tests can exercise it with in-memory buffers.
pub fn serve<R: BufRead, W: Write>(reader: R, mut writer: W) -> std::io::Result<()> {
    for line in reader.lines() {
        let line = line?;
        if line.trim().is_empty() {
            continue;
        }

        // Malformed input is skipped rather than allowed to tear down the
        // server; a stdio client may recover and send a valid message next.
        let Ok(request) = serde_json::from_str::<Request>(&line) else {
            continue;
        };

        if let Some(response) = handle(&request) {
            let encoded = serde_json::to_string(&response)?;
            writer.write_all(encoded.as_bytes())?;
            writer.write_all(b"\n")?;
            writer.flush()?;
        }
    }

    Ok(())
}

/// Routes one request. Returns `None` for notifications (no `id`), which must
/// not produce a response.
fn handle(request: &Request) -> Option<Response> {
    let id = request.id.clone()?;

    let response = match request.method.as_str() {
        "initialize" => Response::success(id, initialize_result(&request.params)),
        "ping" => Response::success(id, json!({})),
        "tools/list" => Response::success(id, tools_list_result()),
        "tools/call" => handle_tools_call(id, &request.params),
        other => Response::error(id, METHOD_NOT_FOUND, format!("method not found: {other}")),
    };

    Some(response)
}

fn initialize_result(params: &Value) -> Value {
    // Echo the client's requested protocol version when present; this server
    // implements a minimal, version-agnostic subset (initialize/tools).
    let protocol_version = params
        .get("protocolVersion")
        .and_then(Value::as_str)
        .unwrap_or(DEFAULT_PROTOCOL_VERSION);

    json!({
        "protocolVersion": protocol_version,
        "capabilities": { "tools": {} },
        "serverInfo": { "name": SERVER_NAME, "version": SERVER_VERSION }
    })
}

fn tools_list_result() -> Value {
    json!({
        "tools": [
            review_change::definition(),
            scan::definition(),
            context::definition(),
            explain_file::definition(),
        ]
    })
}

fn handle_tools_call(id: Value, params: &Value) -> Response {
    let name = params
        .get("name")
        .and_then(Value::as_str)
        .unwrap_or_default();
    let arguments = params
        .get("arguments")
        .cloned()
        .unwrap_or_else(|| json!({}));

    let outcome = match name {
        review_change::TOOL_NAME => review_change::call(&arguments),
        scan::TOOL_NAME => scan::call(&arguments),
        context::TOOL_NAME => context::call(&arguments),
        explain_file::TOOL_NAME => explain_file::call(&arguments),
        other => Err(format!("unknown tool: {other}")),
    };

    Response::success(id, tool_result(outcome))
}

/// Wraps a tool outcome in an MCP `tools/call` result. Failures are returned
/// in-band as `isError: true` text, per MCP convention, so the agent sees them
/// as tool output rather than a transport-level error.
fn tool_result(outcome: Result<String, String>) -> Value {
    match outcome {
        Ok(text) => json!({ "content": [{ "type": "text", "text": text }], "isError": false }),
        Err(message) => {
            json!({ "content": [{ "type": "text", "text": message }], "isError": true })
        }
    }
}
