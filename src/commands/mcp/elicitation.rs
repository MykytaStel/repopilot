use super::jsonrpc::IncomingResponse;
use super::message_writer::write_message;
use repopilot::verification::CancellationToken;
use serde_json::{Value, json};
use std::collections::HashMap;
use std::io::Write;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::mpsc::{self, Receiver, RecvTimeoutError, Sender};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

const RESPONSE_POLL: Duration = Duration::from_millis(50);

#[derive(Debug, PartialEq, Eq)]
pub(super) enum ElicitationFailure {
    Cancelled,
    TimedOut,
    InvalidResponse,
    Transport,
}

#[derive(Default)]
pub(super) struct ElicitationBroker {
    next_id: AtomicU64,
    pending: Mutex<HashMap<String, Sender<IncomingResponse>>>,
}

impl ElicitationBroker {
    pub(super) fn resolve(&self, response: IncomingResponse) -> bool {
        let key = response
            .id
            .as_str()
            .map(str::to_string)
            .unwrap_or_else(|| response.id.to_string());
        let sender = self
            .pending
            .lock()
            .ok()
            .and_then(|mut pending| pending.remove(&key));
        sender.is_some_and(|sender| sender.send(response).is_ok())
    }

    pub(super) fn request_form<W: Write>(
        &self,
        writer: &Arc<Mutex<&mut W>>,
        params: Value,
        cancellation: &CancellationToken,
        timeout: Duration,
    ) -> Result<Value, ElicitationFailure> {
        if cancellation.is_cancelled() {
            return Err(ElicitationFailure::Cancelled);
        }
        let (id, receiver) = self.register_pending();
        let request = json!({
            "jsonrpc": "2.0",
            "id": id,
            "method": "elicitation/create",
            "params": params
        });
        if write_message(writer, &request).is_err() {
            self.remove_pending(&id);
            return Err(ElicitationFailure::Transport);
        }

        let deadline = Instant::now() + timeout;
        let result = loop {
            if cancellation.is_cancelled() {
                break Err(ElicitationFailure::Cancelled);
            }
            let remaining = deadline.saturating_duration_since(Instant::now());
            if remaining.is_zero() {
                break Err(ElicitationFailure::TimedOut);
            }
            match receiver.recv_timeout(remaining.min(RESPONSE_POLL)) {
                Ok(response) if response.error.is_some() => {
                    break Err(ElicitationFailure::InvalidResponse);
                }
                Ok(response) => match response.result {
                    Some(value) => break Ok(value),
                    None => break Err(ElicitationFailure::InvalidResponse),
                },
                Err(RecvTimeoutError::Timeout) => continue,
                Err(RecvTimeoutError::Disconnected) => {
                    break Err(ElicitationFailure::InvalidResponse);
                }
            }
        };
        self.remove_pending(&id);
        result
    }

    fn register_pending(&self) -> (String, Receiver<IncomingResponse>) {
        let number = self.next_id.fetch_add(1, Ordering::Relaxed) + 1;
        let id = format!("repopilot/elicitation/{number}");
        let (sender, receiver) = mpsc::channel();
        if let Ok(mut pending) = self.pending.lock() {
            pending.insert(id.clone(), sender);
        }
        (id, receiver)
    }

    fn remove_pending(&self, id: &str) {
        if let Ok(mut pending) = self.pending.lock() {
            pending.remove(id);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{ElicitationBroker, ElicitationFailure};
    use crate::commands::mcp::jsonrpc::IncomingResponse;
    use repopilot::verification::CancellationToken;
    use serde_json::{Value, json};
    use std::sync::{Arc, Mutex};
    use std::time::Duration;

    #[test]
    fn response_correlation_ignores_unknown_and_late_ids() {
        let broker = ElicitationBroker::default();
        let (id, receiver) = broker.register_pending();
        assert!(!broker.resolve(response("wrong", json!({ "action": "accept" }))));
        assert!(broker.resolve(response(&id, json!({ "action": "decline" }))));
        assert_eq!(
            receiver.recv().expect("correlated response").result,
            Some(json!({ "action": "decline" }))
        );
        assert!(!broker.resolve(response(&id, json!({ "action": "accept" }))));
    }

    #[test]
    fn request_times_out_and_removes_its_pending_id() {
        let broker = ElicitationBroker::default();
        let mut output = Vec::new();
        let writer = Arc::new(Mutex::new(&mut output));
        let result = broker.request_form(
            &writer,
            json!({ "mode": "form" }),
            &CancellationToken::new(),
            Duration::from_millis(1),
        );
        drop(writer);
        assert_eq!(result, Err(ElicitationFailure::TimedOut));
        let request: Value = serde_json::from_slice(&output).expect("request JSON");
        assert_eq!(request["method"], "elicitation/create");
        assert_eq!(broker.pending.lock().expect("pending").len(), 0);
    }

    #[test]
    fn already_cancelled_request_does_not_write_a_prompt() {
        let broker = ElicitationBroker::default();
        let mut output = Vec::new();
        let writer = Arc::new(Mutex::new(&mut output));
        let cancellation = CancellationToken::new();
        cancellation.cancel();
        assert_eq!(
            broker.request_form(&writer, json!({}), &cancellation, Duration::from_secs(1)),
            Err(ElicitationFailure::Cancelled)
        );
        drop(writer);
        assert!(output.is_empty());
    }

    fn response(id: &str, result: Value) -> IncomingResponse {
        IncomingResponse {
            id: Value::String(id.to_string()),
            result: Some(result),
            error: None,
        }
    }
}
