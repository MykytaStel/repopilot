use repopilot::verification::CancellationToken;
use std::collections::{HashMap, HashSet};

#[derive(Default)]
pub(super) struct RequestRegistry {
    pending: HashSet<String>,
    active: HashMap<String, CancellationToken>,
}

impl RequestRegistry {
    pub(super) fn register(&mut self, key: String, token: CancellationToken) {
        if self.pending.remove(&key) {
            token.cancel();
        }
        if let Some(replaced) = self.active.insert(key, token) {
            replaced.cancel();
        }
    }

    pub(super) fn cancel(&mut self, key: &str) {
        if let Some(token) = self.active.get(key) {
            token.cancel();
        } else {
            self.pending.insert(key.to_string());
        }
    }

    pub(super) fn finish(&mut self, key: &str) {
        self.pending.remove(key);
        self.active.remove(key);
    }
}

#[cfg(test)]
mod tests;
