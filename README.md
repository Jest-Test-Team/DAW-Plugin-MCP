# DAW Agent MCP

Rust MCP daemon + VST/CLAP plugin shell. Spectral analysis in **Julia**. Tests in **Robot Framework**.

## Quick start

```bash
cargo test --workspace
cargo run -p daw-mcp -- --host=reaper
```

Ableton is included in this wave (`--host=ableton`).

Subscription users add the binary as an MCP server in Claude Code / Cursor / Codex. See [docs/mcp-hosts.md](docs/mcp-hosts.md).
