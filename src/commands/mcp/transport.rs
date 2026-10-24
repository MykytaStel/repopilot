#[cfg(test)]
use super::DEFAULT_MAX_RESPONSE_BYTES;
mod dispatch;
use self::dispatch::serve_request_lines;
use super::elicitation::ElicitationBroker;
use super::request_registry::RequestRegistry;
use super::worker::{ToolJob, run_tool_worker};
use super::{ServerState, TOOL_QUEUE_CAPACITY};
use crate::cli::McpOptions;
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
    mut reader: R,
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
    let elicitation = Arc::new(ElicitationBroker::default());
    let writer = Arc::new(Mutex::new(&mut writer));
    let (jobs_tx, jobs_rx) = mpsc::sync_channel::<ToolJob>(TOOL_QUEUE_CAPACITY);

    std::thread::scope(|scope| -> std::io::Result<()> {
        let worker_state = Arc::clone(&state);
        let worker_registry = Arc::clone(&registry);
        let worker_elicitation = Arc::clone(&elicitation);
        let worker_writer = Arc::clone(&writer);
        let worker = scope.spawn(move || {
            run_tool_worker(
                jobs_rx,
                &worker_state,
                &worker_registry,
                &worker_elicitation,
                &worker_writer,
            )
        });
        let serve_result = serve_request_lines(
            &mut reader,
            &writer,
            &state,
            &registry,
            &elicitation,
            &jobs_tx,
        );
        drop(jobs_tx);
        let worker_result = worker
            .join()
            .map_err(|_| std::io::Error::other("MCP tool worker panicked"))?;
        serve_result?;
        worker_result?;
        Ok(())
    })
}
