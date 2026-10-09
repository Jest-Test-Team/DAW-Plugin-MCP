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

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ServeMode {
    Stdio,
    Http(u16),
    /// Plugin IPC only. Must not read stdin (closed stdin would exit a stdio loop).
    Sidecar,
}

pub fn parse_serve_mode(args: &[String]) -> ServeMode {
    if args.iter().any(|a| a == "--sidecar") {
        return ServeMode::Sidecar;
    }
    if let Some(port) = args
        .iter()
        .find_map(|a| a.strip_prefix("--http=").and_then(|p| p.parse().ok()))
    {
        return ServeMode::Http(port);
    }
    ServeMode::Stdio
}

pub fn parse_host_name(args: &[String]) -> String {
    args.iter()
        .find_map(|a| a.strip_prefix("--host=").map(str::to_string))
        .or_else(|| std::env::var("DAW_HOST").ok())
        .unwrap_or_else(|| "reaper".into())
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

    #[test]
    fn sidecar_flag_skips_stdio() {
        use super::{parse_host_name, parse_serve_mode, ServeMode};
        assert_eq!(
            parse_serve_mode(&["daw-mcp".into(), "--sidecar".into()]),
            ServeMode::Sidecar
        );
        assert_eq!(
            parse_serve_mode(&["daw-mcp".into(), "--sidecar".into(), "--http=9".into()]),
            ServeMode::Sidecar
        );
        assert_eq!(
            parse_serve_mode(&["daw-mcp".into(), "--http=8765".into()]),
            ServeMode::Http(8765)
        );
        assert_eq!(parse_serve_mode(&["daw-mcp".into()]), ServeMode::Stdio);
        assert_eq!(
            parse_host_name(&["daw-mcp".into(), "--host=logic".into()]),
            "logic"
        );
    }
}
