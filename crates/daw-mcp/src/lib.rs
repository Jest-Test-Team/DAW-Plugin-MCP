//! Library surface for the MCP daemon (fuzz + HTTP helpers). The CLI lives in `main.rs`.

pub mod host;
pub mod http;
pub mod mcp;
pub mod plugin_ipc;
pub mod providers;
pub mod tools;

use serde_json::Value;

/// Parse a JSON-RPC body. Used by HTTP, stdio, and cargo-fuzz. Never logs secrets.
pub fn parse_jsonrpc(body: &str) -> Result<Value, serde_json::Error> {
    serde_json::from_str(body.trim())
}

#[cfg(test)]
mod tests {
    use super::parse_jsonrpc;

    #[test]
    fn parse_jsonrpc_accepts_initialize() {
        let v = parse_jsonrpc(r#"{"jsonrpc":"2.0","id":1,"method":"initialize"}"#).unwrap();
        assert_eq!(v["method"], "initialize");
    }

    #[test]
    fn parse_jsonrpc_rejects_garbage() {
        assert!(parse_jsonrpc("not-json").is_err());
    }
}
