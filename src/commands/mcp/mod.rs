//! `repopilot mcp` — a local Model Context Protocol server over stdio.
//!
//! The server reads newline-delimited JSON-RPC 2.0 on the main thread and sends
//! tool calls through one standard-library worker thread. That keeps
//! cancellation and progress responsive without an async runtime, while
//! preserving RepoPilot's local-first promise (nothing is uploaded; no AI
//! service is called).

mod context;
mod explain_file;
mod explain_finding;
mod jsonrpc;
mod review_change;
mod scan;
mod scan_cache;

#[cfg(test)]
mod tests;
#[cfg(test)]
mod workspace_freshness_tests;

use crate::cli::McpOptions;
use jsonrpc::{METHOD_NOT_FOUND, Request, Response};
use serde::Serialize;
use serde_json::{Value, json};
use std::collections::HashSet;
use std::io::{BufRead, BufReader, Write};
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex, mpsc};

const SERVER_NAME: &str = "repopilot";
const SERVER_VERSION: &str = env!("CARGO_PKG_VERSION");
const LATEST_PROTOCOL_VERSION: &str = "2025-11-25";
const SUPPORTED_PROTOCOL_VERSIONS: &[&str] = &[LATEST_PROTOCOL_VERSION, "2024-11-05"];
const PARSE_ERROR: i32 = -32700;

#[derive(Default)]
struct ServerState {
    root: PathBuf,
    negotiated: bool,
    initialized: bool,
    last_scan: Option<String>,
    last_review: Option<String>,
}

pub fn run(options: McpOptions) -> Result<(), Box<dyn std::error::Error>> {
    let stdin = std::io::stdin();
    let stdout = std::io::stdout();
    serve_with_root(BufReader::new(stdin), stdout, options.root)?;
    Ok(())
}

/// Drives the JSON-RPC loop until the input stream closes. Generic over the
/// reader and writer so tests can exercise it with in-memory buffers.
#[cfg_attr(not(test), allow(dead_code))]
pub fn serve<R: BufRead, W: Write + Send>(reader: R, writer: W) -> std::io::Result<()> {
    serve_with_root(reader, writer, PathBuf::from("."))
}

pub fn serve_with_root<R: BufRead, W: Write + Send>(
    reader: R,
    mut writer: W,
    root: PathBuf,
) -> std::io::Result<()> {
    let root = root.canonicalize().unwrap_or(root);
    let state = Arc::new(Mutex::new(ServerState {
        root,
        ..ServerState::default()
    }));
    let cancelled = Arc::new(Mutex::new(HashSet::<String>::new()));
    let writer = Arc::new(Mutex::new(&mut writer));
    let (jobs_tx, jobs_rx) = mpsc::channel::<ToolJob>();

    std::thread::scope(|scope| -> std::io::Result<()> {
        let worker_state = Arc::clone(&state);
        let worker_cancelled = Arc::clone(&cancelled);
        let worker_writer = Arc::clone(&writer);
        let worker = scope.spawn(move || {
            run_tool_worker(jobs_rx, &worker_state, &worker_cancelled, &worker_writer)
        });

        for line in reader.lines() {
            let line = line?;
            if line.trim().is_empty() {
                continue;
            }

            let Ok(request) = serde_json::from_str::<Request>(&line) else {
                write_message(
                    &writer,
                    &Response::error(Value::Null, PARSE_ERROR, "parse error"),
                )?;
                continue;
            };

            if request.method == "notifications/cancelled" {
                if let Some(request_id) = cancellation_request_id(&request.params) {
                    cancelled
                        .lock()
                        .map_err(lock_error)?
                        .insert(request_key(&request_id));
                }
                continue;
            }

            if request.method == "tools/call"
                && let Some(id) = request.id.clone()
            {
                let initialized = state.lock().map_err(lock_error)?.initialized;
                if !initialized {
                    write_message(
                        &writer,
                        &Response::error(id, -32002, "MCP server is not initialized"),
                    )?;
                    continue;
                }
                let progress_token = request
                    .params
                    .get("_meta")
                    .and_then(|meta| meta.get("progressToken"))
                    .cloned();
                jobs_tx
                    .send(ToolJob {
                        id,
                        params: request.params,
                        progress_token,
                    })
                    .map_err(|_| {
                        std::io::Error::new(
                            std::io::ErrorKind::BrokenPipe,
                            "MCP tool worker stopped",
                        )
                    })?;
                continue;
            }

            let response = {
                let mut state = state.lock().map_err(lock_error)?;
                handle(&request, &mut state)
            };
            if let Some(response) = response {
                write_message(&writer, &response)?;
            }
        }

        drop(jobs_tx);
        worker
            .join()
            .map_err(|_| std::io::Error::other("MCP tool worker panicked"))??;
        Ok(())
    })
}

struct ToolJob {
    id: Value,
    params: Value,
    progress_token: Option<Value>,
}

fn write_message<W: Write, T: Serialize>(
    writer: &Arc<Mutex<&mut W>>,
    message: &T,
) -> std::io::Result<()> {
    let encoded = serde_json::to_string(message)?;
    let mut writer = writer.lock().map_err(lock_error)?;
    writer.write_all(encoded.as_bytes())?;
    writer.write_all(b"\n")?;
    writer.flush()
}

fn run_tool_worker<W: Write>(
    jobs: mpsc::Receiver<ToolJob>,
    state: &Arc<Mutex<ServerState>>,
    cancelled: &Arc<Mutex<HashSet<String>>>,
    writer: &Arc<Mutex<&mut W>>,
) -> std::io::Result<()> {
    for job in jobs {
        let key = request_key(&job.id);
        if cancelled.lock().map_err(lock_error)?.contains(&key) {
            write_message(
                writer,
                &Response::error(job.id, -32800, "request cancelled"),
            )?;
            continue;
        }

        if let Some(token) = &job.progress_token {
            write_message(writer, &progress_notification(token, 0, "analysis started"))?;
        }

        let mut response = {
            let mut state = state.lock().map_err(lock_error)?;
            handle_tools_call(job.id.clone(), &job.params, &mut state)
        };

        if cancelled.lock().map_err(lock_error)?.remove(&key) {
            response = Response::error(job.id, -32800, "request cancelled");
        } else if let Some(token) = &job.progress_token {
            write_message(
                writer,
                &progress_notification(token, 1, "analysis complete"),
            )?;
        }
        write_message(writer, &response)?;
    }
    Ok(())
}

fn progress_notification(token: &Value, progress: u8, message: &str) -> Value {
    json!({
        "jsonrpc": "2.0",
        "method": "notifications/progress",
        "params": {
            "progressToken": token,
            "progress": progress,
            "total": 1,
            "message": message
        }
    })
}

fn cancellation_request_id(params: &Value) -> Option<Value> {
    params
        .get("requestId")
        .or_else(|| params.get("id"))
        .cloned()
}

fn request_key(id: &Value) -> String {
    id.to_string()
}

fn lock_error<T>(_: std::sync::PoisonError<T>) -> std::io::Error {
    std::io::Error::other("MCP server state lock was poisoned")
}

/// Routes one request. Returns `None` for notifications (no `id`), which must
/// not produce a response.
fn handle(request: &Request, state: &mut ServerState) -> Option<Response> {
    if request.id.is_none() {
        if request.method == "notifications/initialized" && state.negotiated {
            state.initialized = true;
        }
        return None;
    }
    let id = request.id.clone()?;

    if !state.initialized && request.method != "initialize" && request.method != "ping" {
        return Some(Response::error(id, -32002, "MCP server is not initialized"));
    }

    let response = match request.method.as_str() {
        "initialize" => {
            state.negotiated = true;
            state.initialized = false;
            Response::success(id, initialize_result(&request.params))
        }
        "ping" => Response::success(id, json!({})),
        "tools/list" => Response::success(id, tools_list_result()),
        "tools/call" => handle_tools_call(id, &request.params, state),
        "resources/list" => Response::success(id, resources_list_result(state)),
        "resources/read" => handle_resource_read(id, &request.params, state),
        "prompts/list" => Response::success(id, prompts_list_result()),
        "prompts/get" => handle_prompt_get(id, &request.params),
        other => Response::error(id, METHOD_NOT_FOUND, format!("method not found: {other}")),
    };

    Some(response)
}

fn initialize_result(params: &Value) -> Value {
    let requested = params
        .get("protocolVersion")
        .and_then(Value::as_str)
        .unwrap_or(LATEST_PROTOCOL_VERSION);
    let protocol_version = if SUPPORTED_PROTOCOL_VERSIONS.contains(&requested) {
        requested
    } else {
        LATEST_PROTOCOL_VERSION
    };

    json!({
        "protocolVersion": protocol_version,
        "capabilities": {
            "tools": { "listChanged": false },
            "resources": { "subscribe": false, "listChanged": false },
            "prompts": { "listChanged": false }
        },
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
            explain_finding::definition(),
        ]
    })
}

fn handle_tools_call(id: Value, params: &Value, state: &mut ServerState) -> Response {
    let name = params
        .get("name")
        .and_then(Value::as_str)
        .unwrap_or_default();
    let mut arguments = params
        .get("arguments")
        .cloned()
        .unwrap_or_else(|| json!({}));
    if let Err(message) = resolve_tool_paths(&mut arguments, &state.root) {
        return Response::success(id, tool_result(name, Err(message)));
    }

    // Workspace-dependent tool results are evaluated on every call.
    // `repopilot_scan` owns its separate persistent cache with validated
    // Git/config/input fingerprints.
    let outcome = match name {
        review_change::TOOL_NAME => review_change::call(&arguments),
        scan::TOOL_NAME => scan::call(&arguments),
        context::TOOL_NAME => context::call(&arguments),
        explain_file::TOOL_NAME => explain_file::call(&arguments, &state.root),
        explain_finding::TOOL_NAME => explain_finding::call(
            &arguments,
            &state.root,
            state.last_scan.as_deref(),
            state.last_review.as_deref(),
        ),
        other => Err(format!("unknown tool: {other}")),
    };

    if let Ok(text) = &outcome {
        match name {
            scan::TOOL_NAME => state.last_scan = Some(text.clone()),
            review_change::TOOL_NAME => state.last_review = Some(text.clone()),
            _ => {}
        }
    }
    Response::success(id, tool_result(name, outcome))
}

/// Wraps a tool outcome in an MCP `tools/call` result. Failures are returned
/// in-band as `isError: true` text, per MCP convention, so the agent sees them
/// as tool output rather than a transport-level error.
fn tool_result(name: &str, outcome: Result<String, String>) -> Value {
    match outcome {
        Ok(text) => {
            let structured = serde_json::from_str::<Value>(&text).unwrap_or_else(|_| {
                if name == context::TOOL_NAME {
                    json!({ "markdown": text })
                } else {
                    json!({ "text": text })
                }
            });
            json!({
                "content": [{ "type": "text", "text": text }],
                "structuredContent": structured,
                "isError": false
            })
        }
        Err(message) => {
            json!({ "content": [{ "type": "text", "text": message }], "isError": true })
        }
    }
}

fn resolve_tool_paths(arguments: &mut Value, root: &Path) -> Result<(), String> {
    for key in ["path", "config", "baseline"] {
        let Some(value) = arguments.get(key).and_then(Value::as_str) else {
            continue;
        };
        let candidate = if Path::new(value).is_absolute() {
            PathBuf::from(value)
        } else {
            root.join(value)
        };
        let resolved = candidate.canonicalize().unwrap_or(candidate);
        if !resolved.starts_with(root) {
            return Err(format!(
                "`{key}` must stay within MCP root {}",
                root.display()
            ));
        }
        arguments[key] = Value::String(resolved.to_string_lossy().to_string());
    }
    Ok(())
}

fn resources_list_result(state: &ServerState) -> Value {
    let mut resources = vec![
        json!({
            "uri": "repopilot://rules",
            "name": "RepoPilot rule catalog",
            "mimeType": "application/json"
        }),
        json!({
            "uri": "repopilot://repository-summary",
            "name": "RepoPilot repository summary",
            "mimeType": "application/json"
        }),
    ];
    if state.last_scan.is_some() {
        resources.push(json!({
            "uri": "repopilot://last-scan",
            "name": "Last RepoPilot scan",
            "mimeType": "application/json"
        }));
    }
    if state.last_review.is_some() {
        resources.push(json!({
            "uri": "repopilot://last-review",
            "name": "Last RepoPilot review",
            "mimeType": "application/json"
        }));
    }
    json!({ "resources": resources })
}

fn handle_resource_read(id: Value, params: &Value, state: &ServerState) -> Response {
    let uri = params
        .get("uri")
        .and_then(Value::as_str)
        .unwrap_or_default();
    let text = match uri {
        "repopilot://rules" => {
            let rules = repopilot::rules::all_rule_metadata()
                .map(|rule| {
                    json!({
                        "rule_id": rule.rule_id,
                        "title": rule.title,
                        "category": rule.category.label(),
                        "severity": rule.default_severity.label(),
                        "max_severity": rule.severity_ceiling().label(),
                        "confidence": rule.default_confidence.label(),
                        "max_confidence": rule.confidence_ceiling().label(),
                        "lifecycle": rule.lifecycle.label(),
                        "signal_source": rule.signal_source.label(),
                        "docs_url": rule.docs_url
                    })
                })
                .collect::<Vec<_>>();
            serde_json::to_string_pretty(&rules).unwrap_or_else(|_| "[]".to_string())
        }
        "repopilot://last-scan" => state.last_scan.clone().unwrap_or_default(),
        "repopilot://last-review" => state.last_review.clone().unwrap_or_default(),
        "repopilot://repository-summary" => serde_json::to_string_pretty(&json!({
            "root": state.root.to_string_lossy(),
            "git_repository": state.root.join(".git").exists(),
            "config_present": state.root.join("repopilot.toml").is_file(),
            "baseline_present": state.root.join(".repopilot/baseline.json").is_file(),
            "feedback_present": state.root.join(".repopilot/feedback.yml").is_file(),
            "last_scan_available": state.last_scan.is_some(),
            "last_review_available": state.last_review.is_some()
        }))
        .unwrap_or_else(|_| "{}".to_string()),
        _ => {
            return Response::error(id, -32002, format!("resource not found: {uri}"));
        }
    };
    Response::success(
        id,
        json!({ "contents": [{ "uri": uri, "mimeType": "application/json", "text": text }] }),
    )
}

fn prompts_list_result() -> Value {
    json!({
        "prompts": [
            {
                "name": "review-change",
                "description": "Review the current change with RepoPilot evidence."
            },
            {
                "name": "fix-top-risk",
                "description": "Plan the smallest fix for the highest-priority RepoPilot risk."
            }
        ]
    })
}

fn handle_prompt_get(id: Value, params: &Value) -> Response {
    let name = params
        .get("name")
        .and_then(Value::as_str)
        .unwrap_or_default();
    let text = match name {
        "review-change" => {
            "Call repopilot_review_change, inspect definitely-sensitive signals first, and report evidence without claiming the change is safe."
        }
        "fix-top-risk" => {
            "Call repopilot_scan, select the highest-priority evidence-backed finding, and propose the smallest verified remediation."
        }
        _ => return Response::error(id, -32602, format!("unknown prompt: {name}")),
    };
    Response::success(
        id,
        json!({
            "description": text,
            "messages": [{
                "role": "user",
                "content": { "type": "text", "text": text }
            }]
        }),
    )
}
