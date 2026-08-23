//! `repopilot mcp` — a local Model Context Protocol server over stdio.
//!
//! The server reads newline-delimited JSON-RPC 2.0 on the main thread and sends
//! tool calls through one standard-library worker thread. That keeps
//! cancellation and progress responsive without an async runtime, while
//! preserving RepoPilot's local-first promise (nothing is uploaded; no AI
//! service is called).

mod analysis_store;
mod catalog;
mod context;
mod explain_file;
mod explain_finding;
mod explain_review_signal;
mod jsonrpc;
mod progress;
mod publication;
mod request_registry;
mod review_change;
mod review_projection;
mod scan;
mod scan_cache;
mod tool_call;
mod worker;

#[cfg(test)]
mod tests;
#[cfg(test)]
mod workspace_freshness_tests;

use crate::cli::McpOptions;
use analysis_store::AnalysisStore;
use catalog::{
    handle_prompt_get, handle_resource_read, prompts_list_result, resources_list_result,
    tools_list_result,
};
use jsonrpc::{
    INVALID_REQUEST, METHOD_NOT_FOUND, PARSE_ERROR, Request, RequestParseError, Response,
    parse_request,
};
#[cfg(test)]
use publication::tool_result;
use request_registry::RequestRegistry;
use serde_json::{Value, json};
use std::io::{BufRead, BufReader, Write};
use std::path::PathBuf;
use std::sync::{Arc, Mutex, mpsc};
use tool_call::handle_tools_call;
use worker::{ToolJob, enqueue_tool_job, run_tool_worker, write_message};

const SERVER_NAME: &str = "repopilot";
const SERVER_VERSION: &str = env!("CARGO_PKG_VERSION");
const LATEST_PROTOCOL_VERSION: &str = "2025-11-25";
const SUPPORTED_PROTOCOL_VERSIONS: &[&str] = &[LATEST_PROTOCOL_VERSION, "2024-11-05"];
const DEFAULT_MAX_RESPONSE_BYTES: usize = 1_048_576;
const TOOL_QUEUE_CAPACITY: usize = 8;

struct ServerState {
    root: PathBuf,
    negotiated: bool,
    initialized: bool,
    last_scan: Option<String>,
    last_review: Option<String>,
    analyses: AnalysisStore,
    max_response_bytes: usize,
}

impl Default for ServerState {
    fn default() -> Self {
        Self {
            root: PathBuf::new(),
            negotiated: false,
            initialized: false,
            last_scan: None,
            last_review: None,
            analyses: AnalysisStore::default(),
            max_response_bytes: DEFAULT_MAX_RESPONSE_BYTES,
        }
    }
}

pub fn run(options: McpOptions) -> Result<(), Box<dyn std::error::Error>> {
    let stdin = std::io::stdin();
    let stdout = std::io::stdout();
    serve_with_options(
        BufReader::new(stdin),
        stdout,
        options.root,
        options.max_response_bytes,
    )?;
    Ok(())
}

/// Drives the JSON-RPC loop until the input stream closes. Generic over the
/// reader and writer so tests can exercise it with in-memory buffers.
#[cfg_attr(not(test), allow(dead_code))]
pub fn serve<R: BufRead, W: Write + Send>(reader: R, writer: W) -> std::io::Result<()> {
    serve_with_options(
        reader,
        writer,
        PathBuf::from("."),
        DEFAULT_MAX_RESPONSE_BYTES,
    )
}

fn serve_with_options<R: BufRead, W: Write + Send>(
    reader: R,
    mut writer: W,
    root: PathBuf,
    max_response_bytes: usize,
) -> std::io::Result<()> {
    let root = root.canonicalize().unwrap_or(root);
    let state = Arc::new(Mutex::new(ServerState {
        root,
        max_response_bytes,
        ..ServerState::default()
    }));
    let registry = Arc::new(Mutex::new(RequestRegistry::default()));
    let writer = Arc::new(Mutex::new(&mut writer));
    let (jobs_tx, jobs_rx) = mpsc::sync_channel::<ToolJob>(TOOL_QUEUE_CAPACITY);

    std::thread::scope(|scope| -> std::io::Result<()> {
        let mut initialized = false;
        let worker_state = Arc::clone(&state);
        let worker_registry = Arc::clone(&registry);
        let worker_writer = Arc::clone(&writer);
        let worker = scope.spawn(move || {
            run_tool_worker(jobs_rx, &worker_state, &worker_registry, &worker_writer)
        });

        for line in reader.lines() {
            let line = line?;
            if line.trim().is_empty() {
                continue;
            }

            let request = match parse_request(&line) {
                Ok(request) => request,
                Err(RequestParseError::Parse) => {
                    write_message(
                        &writer,
                        &Response::error(Value::Null, PARSE_ERROR, "parse error"),
                    )?;
                    continue;
                }
                Err(RequestParseError::InvalidRequest) => {
                    write_message(
                        &writer,
                        &Response::error(Value::Null, INVALID_REQUEST, "invalid request"),
                    )?;
                    continue;
                }
            };

            if request.method == "notifications/cancelled" {
                if let Some(request_id) = cancellation_request_id(&request.params) {
                    registry
                        .lock()
                        .map_err(lock_error)?
                        .cancel(&request_key(&request_id));
                }
                continue;
            }

            if request.method == "tools/call"
                && let Some(id) = request.id.clone()
            {
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
                let cancellation = repopilot::verification::CancellationToken::new();
                enqueue_tool_job(
                    &jobs_tx,
                    ToolJob {
                        id,
                        params: request.params,
                        progress_token,
                        cancellation,
                    },
                    &registry,
                    &writer,
                )?;
                continue;
            }

            let response = {
                let mut state = state.lock().map_err(lock_error)?;
                let response = handle(&request, &mut state);
                initialized = state.initialized;
                response
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
