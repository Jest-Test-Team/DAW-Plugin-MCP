//! Length-prefixed JSON-RPC for the plugin editor. Never call this from an audio callback.

use crate::mcp;
use crate::tools::ServerState;
use serde_json::{json, Value};
use std::path::PathBuf;
use std::sync::Arc;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::sync::Mutex;

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

/// If `buf` starts with a complete frame, drain it and return the JSON body.
pub fn pop_frame(buf: &mut Vec<u8>) -> Option<Vec<u8>> {
    if buf.len() < 4 {
        return None;
    }
    let len = u32::from_le_bytes(buf[0..4].try_into().ok()?) as usize;
    if buf.len() < 4 + len {
        return None;
    }
    let body = buf[4..4 + len].to_vec();
    buf.drain(..4 + len);
    Some(body)
}

pub async fn serve(state: Arc<Mutex<ServerState>>) -> anyhow::Result<()> {
    #[cfg(unix)]
    {
        serve_unix(state).await
    }
    #[cfg(windows)]
    {
        serve_windows(state).await
    }
}

#[cfg(unix)]
async fn serve_unix(state: Arc<Mutex<ServerState>>) -> anyhow::Result<()> {
    use tokio::net::UnixListener;
    let path = default_socket_path();
    let _ = std::fs::remove_file(&path);
    let listener = UnixListener::bind(&path)?;
    tracing::info!(path = %path.display(), "plugin ipc listening");
    loop {
        let (stream, _) = listener.accept().await?;
        let state = Arc::clone(&state);
        tokio::spawn(async move {
            if let Err(e) = handle_stream(stream, state).await {
                tracing::debug!(error = %e, "plugin ipc client ended");
            }
        });
    }
}

#[cfg(windows)]
async fn serve_windows(state: Arc<Mutex<ServerState>>) -> anyhow::Result<()> {
    use tokio::net::windows::named_pipe::ServerOptions;
    const PIPE: &str = r"\\.\pipe\daw-mcp";
    let mut server = ServerOptions::new()
        .first_pipe_instance(true)
        .create(PIPE)?;
    tracing::info!("plugin ipc listening on named pipe");
    loop {
        server.connect().await?;
        let connected = server;
        server = ServerOptions::new().create(PIPE)?;
        let state = Arc::clone(&state);
        tokio::spawn(async move {
            if let Err(e) = handle_stream(connected, state).await {
                tracing::debug!(error = %e, "plugin ipc client ended");
            }
        });
    }
}

async fn handle_stream<S>(mut stream: S, state: Arc<Mutex<ServerState>>) -> anyhow::Result<()>
where
    S: AsyncReadExt + AsyncWriteExt + Unpin,
{
    let mut buf = Vec::new();
    let mut tmp = [0u8; 4096];
    loop {
        let n = stream.read(&mut tmp).await?;
        if n == 0 {
            break;
        }
        buf.extend_from_slice(&tmp[..n]);
        while let Some(body) = pop_frame(&mut buf) {
            let reply = dispatch_bytes(&state, &body).await;
            stream.write_all(&encode_frame(&reply)).await?;
        }
    }
    Ok(())
}

async fn dispatch_bytes(state: &Arc<Mutex<ServerState>>, body: &[u8]) -> Vec<u8> {
    let msg: Value = match serde_json::from_slice(body) {
        Ok(v) => v,
        Err(e) => {
            return serde_json::to_vec(&json!({
                "jsonrpc": "2.0",
                "id": null,
                "error": {"code": -32700, "message": e.to_string()}
            }))
            .unwrap_or_else(|_| b"{}".to_vec());
        }
    };
    let id = msg.get("id").cloned().unwrap_or(Value::Null);
    if id.is_null() {
        return b"{}".to_vec();
    }
    let method = msg.get("method").and_then(|m| m.as_str()).unwrap_or("");
    let params = msg.get("params").cloned().unwrap_or(json!({}));
    let mut g = state.lock().await;
    let resp = match mcp::dispatch(&mut g, method, params).await {
        Ok(r) => json!({"jsonrpc": "2.0", "id": id, "result": r}),
        Err(e) => json!({
            "jsonrpc": "2.0",
            "id": id,
            "error": {"code": -32000, "message": e.to_string()}
        }),
    };
    serde_json::to_vec(&resp).unwrap_or_else(|_| b"{}".to_vec())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn frame_roundtrip_pops_one_message() {
        let payload = br#"{"jsonrpc":"2.0","id":1,"method":"ping"}"#;
        let mut buf = encode_frame(payload);
        assert_eq!(pop_frame(&mut buf).as_deref(), Some(payload.as_slice()));
        assert!(buf.is_empty());
    }

    #[test]
    fn pop_frame_waits_for_complete_body() {
        let payload = b"{\"id\":1}";
        let framed = encode_frame(payload);
        let mut buf = framed[..6].to_vec();
        assert!(pop_frame(&mut buf).is_none());
        buf.extend_from_slice(&framed[6..]);
        assert_eq!(pop_frame(&mut buf).unwrap(), payload);
    }

    #[test]
    fn socket_path_is_local() {
        let p = default_socket_path();
        let s = p.to_string_lossy();
        assert!(s.contains("daw-mcp"));
        assert!(!s.contains("~/.claude"));
    }
}
