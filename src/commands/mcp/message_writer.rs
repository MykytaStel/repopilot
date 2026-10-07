use serde::Serialize;
use std::io::Write;
use std::sync::{Arc, Mutex};

pub(super) fn write_message<W: Write, T: Serialize>(
    writer: &Arc<Mutex<&mut W>>,
    message: &T,
) -> std::io::Result<()> {
    let encoded = serde_json::to_string(message)?;
    let mut writer = writer
        .lock()
        .map_err(|_| std::io::Error::other("MCP server state lock was poisoned"))?;
    writer.write_all(encoded.as_bytes())?;
    writer.write_all(b"\n")?;
    writer.flush()
}
