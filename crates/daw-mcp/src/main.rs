//! MCP stdio / loopback HTTP server. Do not bind LLM providers to subscription OAuth.

use anyhow::Context;
use daw_mcp::host::select_host;
use daw_mcp::http;
use daw_mcp::mcp;
use daw_mcp::plugin_ipc;
use daw_mcp::providers;
use daw_mcp::tools;
use std::sync::Arc;
use tokio::sync::Mutex;
use tracing_subscriber::EnvFilter;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(EnvFilter::from_default_env())
        .with_writer(std::io::stderr)
        .init();

    let args: Vec<String> = std::env::args().collect();
    let host_name = args
        .iter()
        .find_map(|a| a.strip_prefix("--host=").map(str::to_string))
        .or_else(|| std::env::var("DAW_HOST").ok())
        .unwrap_or_else(|| "reaper".into());
    let http_port = args
        .iter()
        .find_map(|a| a.strip_prefix("--http=").and_then(|p| p.parse().ok()));
    let provider = providers::LlmProvider::from_env();
    if let Some(p) = &provider {
        tracing::info!(
            endpoint = p.endpoint(),
            oauth = p.uses_subscription_oauth(),
            "llm provider"
        );
    }

    let host = select_host(&host_name).context("select host")?;
    let state = Arc::new(Mutex::new(tools::ServerState::new(host)));
    let ipc_state = Arc::clone(&state);
    tokio::spawn(async move {
        if let Err(e) = plugin_ipc::serve(ipc_state).await {
            tracing::error!(error = %e, "plugin ipc");
        }
    });
    if let Some(port) = http_port {
        http::serve_loopback(state, port).await
    } else {
        mcp::serve_stdio(state).await
    }
}
