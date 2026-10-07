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
mod elicitation;
mod explain_file;
mod explain_finding;
mod explain_review_signal;
mod jsonrpc;
mod message_writer;
mod progress;
mod publication;
mod request_registry;
mod review_change;
mod review_projection;
mod scan;
mod scan_cache;
mod tool_call;
mod tool_paths;
mod transport;
mod verification_consent;
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
use jsonrpc::{METHOD_NOT_FOUND, Request, Response};
#[cfg(test)]
use publication::tool_result;
#[cfg(test)]
use request_registry::RequestRegistry;
use serde_json::{Value, json};
use std::path::PathBuf;
use tool_call::handle_tools_call;
#[cfg(test)]
use transport::serve;
#[cfg(test)]
use worker::{ToolJob, enqueue_tool_job};

const SERVER_NAME: &str = "repopilot";
const SERVER_VERSION: &str = env!("CARGO_PKG_VERSION");
const LATEST_PROTOCOL_VERSION: &str = "2025-11-25";
const SUPPORTED_PROTOCOL_VERSIONS: &[&str] = &[LATEST_PROTOCOL_VERSION, "2024-11-05"];
const DEFAULT_MAX_RESPONSE_BYTES: usize = 1_048_576;
const TOOL_QUEUE_CAPACITY: usize = 8;

struct ServerState {
    root: PathBuf,
    negotiated: bool,
    negotiated_protocol: String,
    elicitation_form: bool,
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
            negotiated_protocol: String::new(),
            elicitation_form: false,
            initialized: false,
            last_scan: None,
            last_review: None,
            analyses: AnalysisStore::default(),
            max_response_bytes: DEFAULT_MAX_RESPONSE_BYTES,
        }
    }
}

pub fn run(options: McpOptions) -> Result<(), Box<dyn std::error::Error>> {
    transport::run(options)
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
            state.negotiated_protocol = negotiated_protocol(&request.params).to_string();
            state.elicitation_form = state.negotiated_protocol == LATEST_PROTOCOL_VERSION
                && request
                    .params
                    .pointer("/capabilities/elicitation/form")
                    .is_some_and(Value::is_object);
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
    let protocol_version = negotiated_protocol(params);

    json!({
        "protocolVersion": protocol_version,
        "capabilities": {
            "tools": { "listChanged": false },
            "resources": { "subscribe": false, "listChanged": false },
            "prompts": { "listChanged": false }
        },
        "serverInfo": { "name": SERVER_NAME, "version": SERVER_VERSION },
        "instructions": catalog::SERVER_INSTRUCTIONS
    })
}

fn negotiated_protocol(params: &Value) -> &str {
    let requested = params
        .get("protocolVersion")
        .and_then(Value::as_str)
        .unwrap_or(LATEST_PROTOCOL_VERSION);
    if SUPPORTED_PROTOCOL_VERSIONS.contains(&requested) {
        requested
    } else {
        LATEST_PROTOCOL_VERSION
    }
}

#[cfg(test)]
mod alias_freshness_tests;
