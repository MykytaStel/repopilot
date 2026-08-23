use super::RequestRegistry;
use repopilot::verification::CancellationToken;

#[test]
fn pending_cancellation_applies_when_request_registers() {
    let mut registry = RequestRegistry::default();
    registry.cancel("7");
    let token = CancellationToken::new();

    registry.register("7".to_string(), token.clone());

    assert!(token.is_cancelled());
    registry.finish("7");
    let reused = CancellationToken::new();
    registry.register("7".to_string(), reused.clone());
    assert!(!reused.is_cancelled());
}

#[test]
fn active_and_repeated_cancellation_are_idempotent() {
    let mut registry = RequestRegistry::default();
    let token = CancellationToken::new();
    registry.register("active".to_string(), token.clone());

    registry.cancel("active");
    registry.cancel("active");

    assert!(token.is_cancelled());
    registry.finish("active");
    let reused = CancellationToken::new();
    registry.register("active".to_string(), reused.clone());
    assert!(!reused.is_cancelled());
}

#[test]
fn finish_clears_pending_and_active_state() {
    let mut registry = RequestRegistry::default();
    registry.cancel("pending");
    registry.finish("pending");
    let pending_reuse = CancellationToken::new();
    registry.register("pending".to_string(), pending_reuse.clone());
    assert!(!pending_reuse.is_cancelled());

    let active = CancellationToken::new();
    registry.register("active".to_string(), active);
    registry.finish("active");
    registry.cancel("active");
    let active_reuse = CancellationToken::new();
    registry.register("active".to_string(), active_reuse.clone());
    assert!(active_reuse.is_cancelled());
}
