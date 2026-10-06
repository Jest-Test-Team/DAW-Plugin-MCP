//! MCP stdio / loopback HTTP server. Do not bind LLM providers to subscription OAuth.

mod host;
mod http;
mod mcp;
mod providers;
mod tools;

use anyhow::Context;
use host::select_host;
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
    let http_port = args.iter().find_map(|a| a.strip_prefix("--http=").and_then(|p| p.parse().ok()));
    let provider = providers::LlmProvider::from_env();
    if let Some(p) = &provider {
        tracing::info!(endpoint = p.endpoint(), oauth = p.uses_subscription_oauth(), "llm provider");
    }

    let host = select_host(&host_name).context("select host")?;
    if let Some(port) = http_port {
        let state = Arc::new(Mutex::new(tools::ServerState::new(host)));
        http::serve_loopback(state, port).await
    } else {
        mcp::serve_stdio(host).await
    }
}
