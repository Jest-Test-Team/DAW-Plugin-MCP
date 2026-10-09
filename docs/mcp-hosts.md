# Connect Claude Code, Cursor, or Codex

Subscription mode: this repo is an **MCP server**. Talk to the official CLI; do not paste Claude Pro/Max OAuth into the plugin.

Use a Release `daw-mcp`, a local `cargo build -p daw-mcp --release`, or the sidecar next to the plugin:

- `DAWAgent.clap/Contents/MacOS/daw-mcp`
- `DAWAgent.vst3/Contents/MacOS/daw-mcp`
- AU: `DAWAgent.component/Contents/PlugIns/DAWAgent.clap/Contents/MacOS/daw-mcp`

## Claude Code

```bash
claude mcp add daw-logic --transport stdio -- /path/to/daw-mcp --host=logic
```

Ableton (this wave):

```bash
claude mcp add daw-ableton --transport stdio -- /path/to/daw-mcp --host=ableton
```

## Codex

`~/.codex/config.toml`:

```toml
[mcp_servers.daw-logic]
command = "/path/to/daw-mcp"
args = ["--host=logic"]
```

## Cursor

Settings → MCP → add a stdio server pointing at the same binary and `--host=logic` (or another host flag).

## Loopback HTTP (optional)

```bash
/path/to/daw-mcp --host=logic --http=8765
```

`--http=8765` still serves loopback HTTP; the plugin socket is always enabled unless you pass `--sidecar` (plugin IPC only, no stdio).

## Plugin chat / API mode

The VST3 / AU / CLAP editor talks to `daw-mcp` over a length-prefixed Unix socket:

- `~/Library/Application Support/DAW-Plugin-MCP/daw-mcp.sock`
- `/tmp/daw-mcp.sock`

The editor **spawns the bundled sidecar** (`daw-mcp --sidecar --host=<saved>`) if nothing is listening. First-time host defaults to **logic** and is remembered in `plugin-ui.json`. You do not need `cargo run`. An already-running daemon is reused (Claude / Codex stdio can share the same plugin socket). Closing the plugin does not kill the sidecar.

Set **your** keys in the plugin Settings panel (or env):

- `ANTHROPIC_API_KEY`
- `OPENAI_API_KEY` / `OPENAI_BASE_URL`
- `OPENROUTER_API_KEY`
- or Ollama at `http://127.0.0.1:11434`

Keys entered in the UI are stored at `~/Library/Application Support/DAW-Plugin-MCP/plugin-ui.json` (mode `0600`). They are never logged.

Forbidden: `~/.claude` session files, `claude -p` as a hidden client, Claude.ai login in the plugin.

Logic Pro: **音訊單元：Apple** is only Apple’s built-in AUs. File **DAW Agent** under Plug-in Manager **類別 → Utility**, then insert from **Utility** (search `daw` works after that). See [README.md](../README.md).

## Host flags

`reaper` `ableton` `bitwig` `logic` `studioone` `protools` `cubase` `flstudio`

## Plugin artifacts

Download VST3/AU from **Actions → Plugin artifacts**. Audio Units are macOS-only; Windows jobs publish VST3. The AU `.component` embeds `DAWAgent.clap` (with `daw-mcp` in `Contents/MacOS`); keep the standalone CLAP from the same `daw-plugin-macos-au` artifact if you also want a CLAP host to load it.
