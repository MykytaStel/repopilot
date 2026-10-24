use super::super::elicitation::ElicitationBroker;
use super::super::jsonrpc::{
    INVALID_REQUEST, IncomingMessage, PARSE_ERROR, RequestParseError, Response, parse_message,
};
use super::super::message_writer::write_message;
use super::super::request_registry::RequestRegistry;
use super::super::worker::{ToolJob, enqueue_tool_job};
use super::super::{ServerState, handle, lock_error, request_key};
use serde_json::Value;
use std::io::{BufRead, Write};
use std::sync::{Arc, Mutex, mpsc};

const MAX_REQUEST_BYTES: usize = 1_048_576;

enum LineReadError {
    TooLarge,
    InvalidUtf8,
}

pub(super) fn serve_request_lines<R: BufRead, W: Write + Send>(
    reader: &mut R,
    writer: &Arc<Mutex<&mut W>>,
    state: &Arc<Mutex<ServerState>>,
    registry: &Arc<Mutex<RequestRegistry>>,
    elicitation: &Arc<ElicitationBroker>,
    jobs_tx: &mpsc::SyncSender<ToolJob>,
) -> std::io::Result<()> {
    let mut initialized = false;
    let mut line = Vec::with_capacity(8 * 1024);
    while let Some(line_text) = read_bounded_line(reader, &mut line)? {
        match line_text {
            Ok(line_text) if !line_text.trim().is_empty() => dispatch_message(
                line_text,
                &mut initialized,
                writer,
                state,
                registry,
                elicitation,
                jobs_tx,
            )?,
            Ok(_) => {}
            Err(error) => write_line_error(writer, error)?,
        }
    }
    Ok(())
}

fn write_line_error<W: Write + Send>(
    writer: &Arc<Mutex<&mut W>>,
    error: LineReadError,
) -> std::io::Result<()> {
    let response = match error {
        LineReadError::TooLarge => Response::error(
            Value::Null,
            INVALID_REQUEST,
            format!("request too large (maximum {MAX_REQUEST_BYTES} bytes)"),
        ),
        LineReadError::InvalidUtf8 => Response::error(Value::Null, PARSE_ERROR, "parse error"),
    };
    write_message(writer, &response)
}

fn dispatch_message<W: Write + Send>(
    line: &str,
    initialized: &mut bool,
    writer: &Arc<Mutex<&mut W>>,
    state: &Arc<Mutex<ServerState>>,
    registry: &Arc<Mutex<RequestRegistry>>,
    elicitation: &Arc<ElicitationBroker>,
    jobs_tx: &mpsc::SyncSender<ToolJob>,
) -> std::io::Result<()> {
    match parse_message(line) {
        Ok(IncomingMessage::Request(request)) => {
            dispatch_request(request, initialized, writer, state, registry, jobs_tx)
        }
        Ok(IncomingMessage::Response(response)) => {
            // A response is useful only while its exact server request is pending.
            elicitation.resolve(response);
            Ok(())
        }
        Err(RequestParseError::Parse) => {
            write_rpc_error(writer, Value::Null, PARSE_ERROR, "parse error")
        }
        Err(RequestParseError::InvalidRequest) => {
            write_rpc_error(writer, Value::Null, INVALID_REQUEST, "invalid request")
        }
    }
}

fn dispatch_request<W: Write + Send>(
    request: super::super::jsonrpc::Request,
    initialized: &mut bool,
    writer: &Arc<Mutex<&mut W>>,
    state: &Arc<Mutex<ServerState>>,
    registry: &Arc<Mutex<RequestRegistry>>,
    jobs_tx: &mpsc::SyncSender<ToolJob>,
) -> std::io::Result<()> {
    if request.method == "notifications/cancelled" {
        if let Some(request_id) = cancellation_request_id(&request.params) {
            registry
                .lock()
                .map_err(lock_error)?
                .cancel(&request_key(&request_id));
        }
        return Ok(());
    }
    let Some(request_id) = request.id.clone() else {
        return handle_notification(&request, initialized, state);
    };
    match request.method.as_str() {
        "ping" => write_message(
            writer,
            &Response::success(request_id, serde_json::json!({})),
        ),
        "tools/call" => {
            enqueue_tool_call(request, request_id, *initialized, jobs_tx, registry, writer)
        }
        _ => handle_sync_request(request, request_id, initialized, state, writer),
    }
}

fn handle_notification(
    request: &super::super::jsonrpc::Request,
    initialized: &mut bool,
    state: &Arc<Mutex<ServerState>>,
) -> std::io::Result<()> {
    if request.method == "notifications/initialized" && !*initialized {
        let mut state = state.lock().map_err(lock_error)?;
        if state.negotiated {
            state.initialized = true;
        }
        *initialized = state.initialized;
    }
    Ok(())
}

fn enqueue_tool_call<W: Write + Send>(
    request: super::super::jsonrpc::Request,
    request_id: Value,
    initialized: bool,
    jobs_tx: &mpsc::SyncSender<ToolJob>,
    registry: &Arc<Mutex<RequestRegistry>>,
    writer: &Arc<Mutex<&mut W>>,
) -> std::io::Result<()> {
    if !initialized {
        return write_rpc_error(writer, request_id, -32002, "MCP server is not initialized");
    }
    let progress_token = request
        .params
        .get("_meta")
        .and_then(|meta| meta.get("progressToken"))
        .cloned();
    enqueue_tool_job(
        jobs_tx,
        ToolJob {
            id: request_id,
            params: request.params,
            progress_token,
            cancellation: repopilot::verification::CancellationToken::new(),
        },
        registry,
        writer,
    )
}

fn handle_sync_request<W: Write + Send>(
    request: super::super::jsonrpc::Request,
    request_id: Value,
    initialized: &mut bool,
    state: &Arc<Mutex<ServerState>>,
    writer: &Arc<Mutex<&mut W>>,
) -> std::io::Result<()> {
    let mut state = match state.try_lock() {
        Ok(state) => state,
        Err(std::sync::TryLockError::WouldBlock) => {
            return write_rpc_error(
                writer,
                request_id,
                -32000,
                "MCP server is busy processing another tool call",
            );
        }
        Err(std::sync::TryLockError::Poisoned(error)) => return Err(lock_error(error)),
    };
    let response = handle(&request, &mut state);
    *initialized = state.initialized;
    if let Some(response) = response {
        write_message(writer, &response)?;
    }
    Ok(())
}

fn write_rpc_error<W: Write + Send>(
    writer: &Arc<Mutex<&mut W>>,
    id: Value,
    code: i32,
    message: impl Into<String>,
) -> std::io::Result<()> {
    write_message(writer, &Response::error(id, code, message))
}

fn read_bounded_line<'a, R: BufRead>(
    reader: &mut R,
    line: &'a mut Vec<u8>,
) -> std::io::Result<Option<Result<&'a str, LineReadError>>> {
    let Some(within_limit) = read_request_line(reader, line)? else {
        return Ok(None);
    };
    if !within_limit {
        return Ok(Some(Err(LineReadError::TooLarge)));
    }
    Ok(Some(
        std::str::from_utf8(line).map_err(|_| LineReadError::InvalidUtf8),
    ))
}

fn read_request_line<R: BufRead>(
    reader: &mut R,
    line: &mut Vec<u8>,
) -> std::io::Result<Option<bool>> {
    line.clear();
    let mut within_limit = true;
    loop {
        let available = reader.fill_buf()?;
        if available.is_empty() {
            return Ok(request_line_eof(within_limit, line.is_empty()));
        }
        let newline = available.iter().position(|byte| *byte == b'\n');
        let content_length = newline.unwrap_or(available.len());
        within_limit = retain_request_chunk(line, &available[..content_length], within_limit);
        reader.consume(content_length + usize::from(newline.is_some()));
        if newline.is_some() {
            return Ok(Some(within_limit));
        }
    }
}

fn request_line_eof(within_limit: bool, line_is_empty: bool) -> Option<bool> {
    if within_limit && line_is_empty {
        None
    } else {
        Some(within_limit)
    }
}

fn retain_request_chunk(line: &mut Vec<u8>, chunk: &[u8], within_limit: bool) -> bool {
    if !within_limit {
        return false;
    }
    if line.len().saturating_add(chunk.len()) > MAX_REQUEST_BYTES {
        line.clear();
        return false;
    }
    line.extend_from_slice(chunk);
    true
}

fn cancellation_request_id(params: &Value) -> Option<Value> {
    params
        .get("requestId")
        .or_else(|| params.get("id"))
        .cloned()
}
