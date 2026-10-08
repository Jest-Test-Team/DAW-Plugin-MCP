use crate::providers::{self, LlmProvider};
use crate::tools::{self, ServerState};
use serde_json::{json, Value};
use std::sync::Arc;
use tokio::io::{AsyncBufReadExt, AsyncReadExt, AsyncWriteExt, BufReader};
use tokio::sync::Mutex;

pub async fn serve_stdio(state: Arc<Mutex<ServerState>>) -> anyhow::Result<()> {
    let stdin = tokio::io::stdin();
    let mut reader = BufReader::new(stdin);
    let mut stdout = tokio::io::stdout();

    loop {
        let msg = match read_message(&mut reader).await? {
            None => break,
            Some(v) => v,
        };
        if msg.get("id").is_none() {
            continue; // notification
        }
        let id = msg.get("id").cloned().unwrap_or(Value::Null);
        let method = msg.get("method").and_then(|m| m.as_str()).unwrap_or("");
        let params = msg.get("params").cloned().unwrap_or(json!({}));
        let result = {
            let mut g = state.lock().await;
            dispatch(&mut g, method, params).await
        };
        let resp = match result {
            Ok(r) => json!({"jsonrpc":"2.0","id":id,"result":r}),
            Err(e) => json!({
                "jsonrpc":"2.0",
                "id":id,
                "error": {"code": -32000, "message": e.to_string()}
            }),
        };
        write_message(&mut stdout, &resp).await?;
    }
    Ok(())
}

pub async fn dispatch(
    state: &mut ServerState,
    method: &str,
    params: Value,
) -> Result<Value, anyhow::Error> {
    match method {
        "initialize" => Ok(json!({
            "protocolVersion": "2025-03-26",
            "capabilities": { "tools": { "listChanged": false } },
            "serverInfo": { "name": "daw-mcp-server", "version": "0.1.0" }
        })),
        "ping" | "plugin/hello" => Ok(json!({
            "ok": true,
            "host": state.host.host_id(),
        })),
        "tools/list" => Ok(json!({ "tools": tools::list_tools() })),
        "tools/call" => {
            let name = params
                .get("name")
                .and_then(|v| v.as_str())
                .ok_or_else(|| anyhow::anyhow!("missing tool name"))?;
            let arguments = params.get("arguments").cloned().unwrap_or(json!({}));
            tools::call(state, name, arguments).await
        }
        "assistant/chat" => assistant_chat(params).await,
        other => Err(anyhow::anyhow!("unknown method {other}")),
    }
}

async fn assistant_chat(params: Value) -> Result<Value, anyhow::Error> {
    let messages = params
        .get("messages")
        .and_then(|m| m.as_array())
        .cloned()
        .unwrap_or_default();
    if messages.is_empty() {
        anyhow::bail!("messages required");
    }
    let provider = params
        .get("provider")
        .cloned()
        .and_then(|v| serde_json::from_value::<LlmProvider>(v).ok())
        .or_else(LlmProvider::from_env)
        .ok_or_else(|| {
            anyhow::anyhow!(
                "no LLM provider; set API key in plugin UI or ANTHROPIC_API_KEY / OPENAI_API_KEY / OPENROUTER_API_KEY / Ollama"
            )
        })?;
    let text = providers::complete(&provider, &messages).await?;
    Ok(json!({"text": text}))
}

async fn read_message(reader: &mut BufReader<tokio::io::Stdin>) -> anyhow::Result<Option<Value>> {
    let mut header = String::new();
    let mut content_length = None;
    loop {
        header.clear();
        let n = reader.read_line(&mut header).await?;
        if n == 0 {
            return Ok(None);
        }
        let line = header.trim_end();
        if line.is_empty() {
            break;
        }
        if let Some(v) = line.to_ascii_lowercase().strip_prefix("content-length:") {
            content_length = Some(v.trim().parse::<usize>()?);
        }
    }
    let len = content_length.ok_or_else(|| anyhow::anyhow!("missing Content-Length"))?;
    let mut buf = vec![0u8; len];
    reader.read_exact(&mut buf).await?;
    Ok(Some(serde_json::from_slice(&buf)?))
}

async fn write_message(stdout: &mut tokio::io::Stdout, value: &Value) -> anyhow::Result<()> {
    let body = serde_json::to_vec(value)?;
    let header = format!("Content-Length: {}\r\n\r\n", body.len());
    stdout.write_all(header.as_bytes()).await?;
    stdout.write_all(&body).await?;
    stdout.flush().await?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use daw_bridge_reaper::mock;

    #[tokio::test]
    async fn ping_returns_host_id() {
        let mut state = ServerState::new(Box::new(mock()));
        let v = dispatch(&mut state, "ping", json!({})).await.unwrap();
        assert_eq!(v["ok"], true);
        assert_eq!(v["host"], "reaper");
    }

    #[tokio::test]
    async fn assistant_chat_requires_messages() {
        let mut state = ServerState::new(Box::new(mock()));
        let err = dispatch(&mut state, "assistant/chat", json!({}))
            .await
            .unwrap_err();
        assert!(err.to_string().contains("messages"));
    }

    #[tokio::test]
    async fn assistant_chat_empty_key_fails_before_http() {
        let mut state = ServerState::new(Box::new(mock()));
        let err = dispatch(
            &mut state,
            "assistant/chat",
            json!({
                "messages":[{"role":"user","content":"hi"}],
                "provider":{"kind":"anthropic_api","api_key":""}
            }),
        )
        .await
        .unwrap_err();
        let msg = err.to_string();
        assert!(msg.contains("empty") || msg.contains("API key"));
        assert!(!msg.contains(".claude"));
    }
}
