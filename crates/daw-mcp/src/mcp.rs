use crate::tools::{self, ServerState};
use daw_contracts::DawHost;
use serde_json::{json, Value};
use tokio::io::{AsyncBufReadExt, AsyncReadExt, AsyncWriteExt, BufReader};

pub async fn serve_stdio(host: Box<dyn DawHost>) -> anyhow::Result<()> {
    let mut state = ServerState::new(host);
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
        let result = dispatch(&mut state, method, params).await;
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

async fn dispatch(state: &mut ServerState, method: &str, params: Value) -> Result<Value, anyhow::Error> {
    match method {
        "initialize" => Ok(json!({
            "protocolVersion": "2025-03-26",
            "capabilities": { "tools": { "listChanged": false } },
            "serverInfo": { "name": "daw-mcp-server", "version": "0.1.0" }
        })),
        "ping" => Ok(json!({})),
        "tools/list" => Ok(json!({ "tools": tools::list_tools() })),
        "tools/call" => {
            let name = params
                .get("name")
                .and_then(|v| v.as_str())
                .ok_or_else(|| anyhow::anyhow!("missing tool name"))?;
            let arguments = params.get("arguments").cloned().unwrap_or(json!({}));
            tools::call(state, name, arguments).await
        }
        other => Err(anyhow::anyhow!("unknown method {other}")),
    }
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
