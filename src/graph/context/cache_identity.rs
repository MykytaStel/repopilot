use sha2::{Digest, Sha256};
use std::path::Path;

pub(super) fn stable_hash_hex(bytes: &[u8]) -> String {
    let digest = Sha256::digest(bytes);
    digest.iter().map(|byte| format!("{byte:02x}")).collect()
}

pub(super) fn resolver_input_fingerprint(root: &Path) -> String {
    let content = crate::graph::resolver::effective_tsconfig_content(root);
    stable_hash_hex(
        serde_json::to_string(&content)
            .expect("config content serializes")
            .as_bytes(),
    )
}
