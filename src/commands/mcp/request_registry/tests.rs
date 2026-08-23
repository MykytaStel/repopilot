use super::RequestRegistry;
use repopilot::verification::CancellationToken;

#[test]
fn unknown_cancellation_is_ignored_instead_of_buffered() {
    let mut registry = RequestRegistry::default();
    assert!(!registry.cancel("7"));
    let token = CancellationToken::new();

    registry.register("7".to_string(), token.clone());

    assert!(!token.is_cancelled());
}

#[test]
fn active_and_repeated_cancellation_are_idempotent() {
    let mut registry = RequestRegistry::default();
    let token = CancellationToken::new();
    registry.register("active".to_string(), token.clone());

    assert!(registry.cancel("active"));
    assert!(registry.cancel("active"));

    assert!(token.is_cancelled());
    registry.finish("active");
    let reused = CancellationToken::new();
    registry.register("active".to_string(), reused.clone());
    assert!(!reused.is_cancelled());
}

#[test]
fn finish_clears_active_state() {
    let mut registry = RequestRegistry::default();
    let active = CancellationToken::new();
    registry.register("active".to_string(), active);
    registry.finish("active");
    assert!(!registry.cancel("active"));
    let active_reuse = CancellationToken::new();
    registry.register("active".to_string(), active_reuse.clone());
    assert!(!active_reuse.is_cancelled());
}
