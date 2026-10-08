//! Loopback Streamable-HTTP style JSON-RPC. Bind 127.0.0.1 only.

use crate::mcp;
use crate::parse_jsonrpc;
use crate::tools::ServerState;
use serde_json::{json, Value};
use std::sync::Arc;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpListener;
use tokio::sync::Mutex;

pub const LOOPBACK_HOST: &str = "127.0.0.1";

pub async fn serve_loopback(state: Arc<Mutex<ServerState>>, port: u16) -> anyhow::Result<()> {
    let listener = TcpListener::bind((LOOPBACK_HOST, port)).await?;
    tracing::info!("daw-mcp HTTP on {LOOPBACK_HOST}:{port}");
    loop {
        let (mut socket, _) = listener.accept().await?;
        let state = Arc::clone(&state);
        tokio::spawn(async move {
            let mut buf = vec![0u8; 65536];
            let mut n = 0usize;
            loop {
                match socket.read(&mut buf[n..]).await {
                    Ok(0) | Err(_) => break,
                    Ok(k) => {
                        n += k;
                        if n >= buf.len() {
                            break;
                        }
                        if buf[..n].windows(4).any(|w| w == b"\r\n\r\n")
                            || buf[..n].windows(2).any(|w| w == b"\n\n")
                        {
                            break;
                        }
                    }
                }
            }
            if n == 0 {
                return;
            }
            let raw = String::from_utf8_lossy(&buf[..n]);
            let reply_bytes = match classify_http(&raw) {
                HttpClass::Health => health_response(),
                HttpClass::OtherGet => {
                    http_status(404, "application/json", b"{\"error\":\"not found\"}")
                }
                HttpClass::Rpc => {
                    let body = http_body(&raw);
                    let reply = match handle_body(&state, body).await {
                        Ok(v) => v,
                        Err(e) => {
                            json!({"jsonrpc":"2.0","id":null,"error":{"code":-32603,"message":e.to_string()}})
                        }
                    };
                    json_response(&reply)
                }
            };
            let _ = socket.write_all(&reply_bytes).await;
        });
    }
}

fn health_response() -> Vec<u8> {
    let payload = format!(r#"{{"ok":true,"bind":"{LOOPBACK_HOST}"}}"#);
    http_status(200, "application/json", payload.as_bytes())
}

fn json_response(value: &Value) -> Vec<u8> {
    let payload = serde_json::to_vec(value).unwrap_or_else(|_| b"{}".to_vec());
    http_status(200, "application/json", &payload)
}

fn http_status(code: u16, content_type: &str, payload: &[u8]) -> Vec<u8> {
    let reason = match code {
        200 => "OK",
        404 => "Not Found",
        _ => "Error",
    };
    let header = format!(
        "HTTP/1.1 {code} {reason}\r\nContent-Type: {content_type}\r\nContent-Length: {}\r\nConnection: close\r\n\r\n",
        payload.len()
    );
    let mut out = header.into_bytes();
    out.extend_from_slice(payload);
    out
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum HttpClass {
    Health,
    OtherGet,
    Rpc,
}

fn request_line(raw: &str) -> &str {
    raw.lines().next().unwrap_or("").trim_end_matches('\r')
}

fn classify_http(raw: &str) -> HttpClass {
    let mut parts = request_line(raw).split_whitespace();
    let method = parts.next().unwrap_or("");
    let target = parts.next().unwrap_or("");
    let path = http_path(target);
    if method.eq_ignore_ascii_case("GET") || method.eq_ignore_ascii_case("HEAD") {
        if path == "/health" || path.starts_with("/health/") {
            HttpClass::Health
        } else {
            HttpClass::OtherGet
        }
    } else {
        HttpClass::Rpc
    }
}

fn http_path(target: &str) -> &str {
    let without_query = target.split('?').next().unwrap_or(target);
    if let Some(rest) = without_query
        .strip_prefix("http://")
        .or_else(|| without_query.strip_prefix("https://"))
    {
        rest.find('/').map(|i| &rest[i..]).unwrap_or("/")
    } else if without_query.is_empty() {
        "/"
    } else {
        without_query
    }
}

fn http_body(raw: &str) -> &str {
    raw.split("\r\n\r\n")
        .nth(1)
        .or_else(|| raw.split("\n\n").nth(1))
        .unwrap_or("")
        .trim()
}

async fn handle_body(state: &Arc<Mutex<ServerState>>, body: &str) -> anyhow::Result<Value> {
    let msg = parse_jsonrpc(body)?;
    let id = msg.get("id").cloned().unwrap_or(Value::Null);
    let method = msg.get("method").and_then(|m| m.as_str()).unwrap_or("");
    let params = msg.get("params").cloned().unwrap_or(json!({}));
    let mut g = state.lock().await;
    match mcp::dispatch(&mut g, method, params).await {
        Ok(result) => Ok(json!({"jsonrpc":"2.0","id":id,"result":result})),
        Err(e) => Ok(json!({
            "jsonrpc":"2.0",
            "id":id,
            "error":{"code":-32000,"message":e.to_string()}
        })),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn health_payload_is_loopback_only() {
        let s = String::from_utf8(health_response()).unwrap();
        assert!(s.contains("127.0.0.1"));
        assert!(!s.contains("0.0.0.0"));
        assert!(s.contains("Content-Length:"));
    }

    #[test]
    fn classifies_urllib_style_health_get() {
        let raw = "GET /health HTTP/1.1\r\nHost: 127.0.0.1:8765\r\n\r\n";
        assert_eq!(classify_http(raw), HttpClass::Health);
    }

    #[test]
    fn classifies_other_get_as_not_found() {
        let raw = "GET /robots.txt HTTP/1.1\r\nHost: 127.0.0.1:8765\r\n\r\n";
        assert_eq!(classify_http(raw), HttpClass::OtherGet);
    }
}
