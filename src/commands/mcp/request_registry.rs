use repopilot::verification::CancellationToken;
use std::collections::HashMap;

#[derive(Default)]
pub(super) struct RequestRegistry {
    active: HashMap<String, CancellationToken>,
}

impl RequestRegistry {
    pub(super) fn register(&mut self, key: String, token: CancellationToken) -> bool {
        if self.active.contains_key(&key) {
            return false;
        }
        self.active.insert(key, token);
        true
    }

    pub(super) fn cancel(&mut self, key: &str) -> bool {
        if let Some(token) = self.active.get(key) {
            token.cancel();
            true
        } else {
            false
        }
    }

    pub(super) fn finish(&mut self, key: &str) {
        self.active.remove(key);
    }
}

#[cfg(test)]
mod tests;
