//! Message-thread IPC client. Never call from `process()`.

use crate::chat::DAEMON_MISSING;
use serde_json::Value;
use std::io::{Read, Write};
use std::path::PathBuf;
use std::time::Duration;

pub fn default_socket_path() -> PathBuf {
    if cfg!(windows) {
        PathBuf::from(r"\\.\pipe\daw-mcp")
    } else {
        std::env::temp_dir().join("daw-mcp.sock")
    }
}

pub fn encode_frame(body: &[u8]) -> Vec<u8> {
    let len = body.len() as u32;
    let mut out = Vec::with_capacity(4 + body.len());
    out.extend_from_slice(&len.to_le_bytes());
    out.extend_from_slice(body);
    out
}

pub fn decode_frame(buf: &[u8]) -> Option<&[u8]> {
    if buf.len() < 4 {
        return None;
    }
    let len = u32::from_le_bytes(buf[0..4].try_into().ok()?) as usize;
    if buf.len() < 4 + len {
        return None;
    }
    Some(&buf[4..4 + len])
}

/// Blocking JSON-RPC over the daemon socket. Editor / worker threads only.
pub fn rpc_call(method: &str, params: Value) -> Result<Value, String> {
    let req = crate::chat::jsonrpc_request(1, method, params);
    let body = serde_json::to_vec(&req).map_err(|e| e.to_string())?;
    let mut stream = connect().map_err(|_| DAEMON_MISSING.to_string())?;
    stream
        .write_all(&encode_frame(&body))
        .map_err(|_| DAEMON_MISSING.to_string())?;
    stream.flush().map_err(|_| DAEMON_MISSING.to_string())?;

    let mut len_buf = [0u8; 4];
    stream
        .read_exact(&mut len_buf)
        .map_err(|_| DAEMON_MISSING.to_string())?;
    let len = u32::from_le_bytes(len_buf) as usize;
    if len == 0 || len > 8 * 1024 * 1024 {
        return Err("rpc response too large".into());
    }
    let mut buf = vec![0u8; len];
    stream.read_exact(&mut buf).map_err(|e| e.to_string())?;
    let v: Value = serde_json::from_slice(&buf).map_err(|e| e.to_string())?;
    if let Some(err) = v.get("error") {
        return Err(err
            .get("message")
            .and_then(|m| m.as_str())
            .unwrap_or("rpc error")
            .to_string());
    }
    Ok(v.get("result").cloned().unwrap_or(Value::Null))
}

fn connect() -> Result<Box<dyn ReadWrite>, std::io::Error> {
    #[cfg(unix)]
    {
        let s = std::os::unix::net::UnixStream::connect(default_socket_path())?;
        let _ = s.set_read_timeout(Some(Duration::from_secs(90)));
        let _ = s.set_write_timeout(Some(Duration::from_secs(10)));
        Ok(Box::new(s))
    }
    #[cfg(windows)]
    {
        let s = std::fs::OpenOptions::new()
            .read(true)
            .write(true)
            .open(default_socket_path())?;
        Ok(Box::new(s))
    }
}

trait ReadWrite: Read + Write {}
impl<T: Read + Write> ReadWrite for T {}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn frame_roundtrip() {
        let payload = b"{\"method\":\"plugin/hello\"}";
        let framed = encode_frame(payload);
        assert_eq!(decode_frame(&framed), Some(payload.as_slice()));
    }

    #[test]
    fn rpc_call_missing_daemon_is_localized() {
        // Unlikely a daemon is bound in unit tests; if one is, still must not mention ~/.claude.
        let err = rpc_call("ping", serde_json::json!({})).err();
        if let Some(e) = err {
            assert!(!e.contains(".claude"));
        }
    }
}
