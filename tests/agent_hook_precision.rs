//! Agent stop hooks stop an agent only for checks or safeguards the session
//! weakened, raise each signal once per session, survive a configured review
//! gate and a long signal list, and keep RepoPilot's session state out of
//! `git status`.
#![cfg(unix)]

#[path = "agent_hook_precision/state.rs"]
mod state;
#[path = "agent_hook_precision/stop.rs"]
mod stop;
#[path = "agent_hook_precision/support.rs"]
mod support;
