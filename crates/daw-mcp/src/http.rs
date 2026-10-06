//! Loopback Streamable-HTTP style JSON-RPC. Bind 127.0.0.1 only.

use crate::tools::{self, ServerState};
use serde_json::{json, Value};
use std::sync::Arc;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpListener;
use tokio::sync::Mutex;

pub async fn serve_loopback(state: Arc<Mutex<ServerState>>, port: u16) -> anyhow::Result<()> {
    let listener = TcpListener::bind(("127.0.0.1", port)).await?;
    tracing::info!("daw-mcp HTTP on 127.0.0.1:{port}");
    loop {
        let (mut socket, _) = listener.accept().await?;
        let state = Arc::clone(&state);
        tokio::spawn(async move {
            let mut buf = vec![0u8; 65536];
            let n = match socket.read(&mut buf).await {
                Ok(0) | Err(_) => return,
                Ok(n) => n,
            };
            let raw = String::from_utf8_lossy(&buf[..n]);
            let body = raw.split("\r\n\r\n").nth(1).unwrap_or("").trim();
            let reply = match handle_body(&state, body).await {
                Ok(v) => v,
                Err(e) => json!({"jsonrpc":"2.0","id":null,"error":{"code":-32603,"message":e.to_string()}}),
            };
            let payload = serde_json::to_vec(&reply).unwrap_or_else(|_| b"{}".to_vec());
            let header = format!(
                "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n",
                payload.len()
            );
            let _ = socket.write_all(header.as_bytes()).await;
            let _ = socket.write_all(&payload).await;
        });
    }
}

async fn handle_body(state: &Arc<Mutex<ServerState>>, body: &str) -> anyhow::Result<Value> {
    let msg: Value = serde_json::from_str(body)?;
    let id = msg.get("id").cloned().unwrap_or(Value::Null);
    let method = msg.get("method").and_then(|m| m.as_str()).unwrap_or("");
    let params = msg.get("params").cloned().unwrap_or(json!({}));
    let mut g = state.lock().await;
    let result = match method {
        "initialize" => json!({
            "protocolVersion": "2025-03-26",
            "capabilities": { "tools": {} },
            "serverInfo": { "name": "daw-mcp-server", "version": "0.1.0" }
        }),
        "tools/list" => json!({ "tools": tools::list_tools() }),
        "tools/call" => {
            let name = params.get("name").and_then(|v| v.as_str()).unwrap_or("");
            let arguments = params.get("arguments").cloned().unwrap_or(json!({}));
            tools::call(&mut g, name, arguments).await?
        }
        other => return Ok(json!({"jsonrpc":"2.0","id":id,"error":{"code":-32601,"message":format!("unknown {other}")}})),
    };
    Ok(json!({"jsonrpc":"2.0","id":id,"result":result}))
}
