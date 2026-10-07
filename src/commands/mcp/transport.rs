#[cfg(test)]
use super::DEFAULT_MAX_RESPONSE_BYTES;
use super::jsonrpc::{
    INVALID_REQUEST, IncomingMessage, PARSE_ERROR, RequestParseError, Response, parse_message,
};
use super::request_registry::RequestRegistry;
use super::worker::{ToolJob, enqueue_tool_job, run_tool_worker, write_message};
use super::{ServerState, TOOL_QUEUE_CAPACITY, handle, lock_error, request_key};
use crate::cli::McpOptions;
use serde_json::Value;
use std::io::{BufRead, BufReader, Write};
use std::path::PathBuf;
use std::sync::{Arc, Mutex, mpsc};

pub(super) fn run(options: McpOptions) -> Result<(), Box<dyn std::error::Error>> {
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

#[cfg(test)]
pub(super) fn serve<R: BufRead, W: Write + Send>(reader: R, writer: W) -> std::io::Result<()> {
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

            let request = match parse_message(&line) {
                Ok(IncomingMessage::Request(request)) => request,
                Ok(IncomingMessage::Response(response)) => {
                    // No server request is pending yet; a late response is inert.
                    let _ = (response.id, response.result, response.error);
                    continue;
                }
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
