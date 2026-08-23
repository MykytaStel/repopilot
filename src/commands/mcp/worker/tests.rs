use super::{ToolJob, run_tool_worker};
use crate::commands::mcp::ServerState;
use crate::commands::mcp::request_registry::RequestRegistry;
use repopilot::verification::CancellationToken;
use serde_json::json;
use std::io::{self, Write};
use std::sync::{Arc, Mutex, mpsc};

struct FailingWriter;

impl Write for FailingWriter {
    fn write(&mut self, _buffer: &[u8]) -> io::Result<usize> {
        Err(io::Error::new(io::ErrorKind::BrokenPipe, "writer closed"))
    }

    fn flush(&mut self) -> io::Result<()> {
        Ok(())
    }
}

#[test]
fn writer_failure_still_cleans_request_registry() {
    let registry = Arc::new(Mutex::new(RequestRegistry::default()));
    let cancellation = CancellationToken::new();
    cancellation.cancel();
    registry
        .lock()
        .expect("registry")
        .register("41".to_string(), cancellation.clone());
    let (sender, receiver) = mpsc::channel();
    sender
        .send(ToolJob {
            id: json!(41),
            params: json!({}),
            progress_token: None,
            cancellation,
        })
        .expect("job");
    drop(sender);
    let state = Arc::new(Mutex::new(ServerState::default()));
    let mut output = FailingWriter;
    let writer = Arc::new(Mutex::new(&mut output));

    let error =
        run_tool_worker(receiver, &state, &registry, &writer).expect_err("closed writer must fail");

    assert_eq!(error.kind(), io::ErrorKind::BrokenPipe);
    let reused = CancellationToken::new();
    let mut registry = registry.lock().expect("registry");
    registry.cancel("41");
    registry.register("41".to_string(), reused.clone());
    assert!(
        reused.is_cancelled(),
        "completed request remained active after writer failure"
    );
}
