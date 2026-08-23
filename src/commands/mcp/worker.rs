use super::jsonrpc::{INVALID_REQUEST, Response};
use super::progress::{ProgressReporter, mode_for_tool_call};
use super::request_registry::RequestRegistry;
use super::{ServerState, lock_error, request_key};
use crate::commands::mcp::tool_call::handle_tools_call_with_context;
use repopilot::verification::CancellationToken;
use serde::Serialize;
use serde_json::Value;
use std::io::Write;
use std::sync::{Arc, Mutex, mpsc};

const SERVER_BUSY: i32 = -32000;

pub(super) struct ToolJob {
    pub id: Value,
    pub params: Value,
    pub progress_token: Option<Value>,
    pub cancellation: CancellationToken,
}

pub(super) fn write_message<W: Write, T: Serialize>(
    writer: &Arc<Mutex<&mut W>>,
    message: &T,
) -> std::io::Result<()> {
    let encoded = serde_json::to_string(message)?;
    let mut writer = writer.lock().map_err(lock_error)?;
    writer.write_all(encoded.as_bytes())?;
    writer.write_all(b"\n")?;
    writer.flush()
}

pub(super) fn enqueue_tool_job<W: Write>(
    jobs_tx: &mpsc::SyncSender<ToolJob>,
    job: ToolJob,
    registry: &Arc<Mutex<RequestRegistry>>,
    writer: &Arc<Mutex<&mut W>>,
) -> std::io::Result<()> {
    let key = request_key(&job.id);
    let registered = registry
        .lock()
        .map_err(lock_error)?
        .register(key.clone(), job.cancellation.clone());
    if !registered {
        return write_message(
            writer,
            &Response::error(job.id, INVALID_REQUEST, "request id is already active"),
        );
    }

    match jobs_tx.try_send(job) {
        Ok(()) => Ok(()),
        Err(mpsc::TrySendError::Full(job)) => {
            registry.lock().map_err(lock_error)?.finish(&key);
            write_message(
                writer,
                &Response::error(job.id, SERVER_BUSY, "MCP tool queue is full; retry later"),
            )
        }
        Err(mpsc::TrySendError::Disconnected(_)) => {
            registry.lock().map_err(lock_error)?.finish(&key);
            Err(std::io::Error::new(
                std::io::ErrorKind::BrokenPipe,
                "MCP tool worker stopped",
            ))
        }
    }
}

pub(super) fn run_tool_worker<W: Write>(
    jobs: mpsc::Receiver<ToolJob>,
    state: &Arc<Mutex<ServerState>>,
    registry: &Arc<Mutex<RequestRegistry>>,
    writer: &Arc<Mutex<&mut W>>,
) -> std::io::Result<()> {
    for job in jobs {
        let key = request_key(&job.id);
        let result = process_job(job, state, writer);
        registry.lock().map_err(lock_error)?.finish(&key);
        result?;
    }
    Ok(())
}

fn process_job<W: Write>(
    job: ToolJob,
    state: &Arc<Mutex<ServerState>>,
    writer: &Arc<Mutex<&mut W>>,
) -> std::io::Result<()> {
    if job.cancellation.is_cancelled() {
        return write_message(
            writer,
            &Response::error(job.id, -32800, "request cancelled"),
        );
    }

    let mode = mode_for_tool_call(&job.params);
    let mut sink = |notification| write_message(writer, &notification);
    let mut reporter = ProgressReporter::new(
        job.progress_token.clone(),
        mode,
        &job.cancellation,
        &mut sink,
    );
    reporter.analysis_started();
    if reporter.has_error() {
        return reporter.into_result();
    }

    let response = {
        let mut state = state.lock().map_err(lock_error)?;
        handle_tools_call_with_context(
            job.id.clone(),
            &job.params,
            &mut state,
            &job.cancellation,
            &mut |event| reporter.verification(event),
        )
    };

    if response.error.is_none()
        && response
            .result
            .as_ref()
            .is_some_and(|result| result["isError"] != true)
    {
        reporter.finish_success();
    }
    reporter.into_result()?;
    write_message(writer, &response)
}

#[cfg(test)]
mod tests;
