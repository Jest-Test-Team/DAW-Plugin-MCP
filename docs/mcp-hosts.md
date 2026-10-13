# Connect Claude Code, Cursor, or Codex

Subscription path: **Desktop / CLI → `daw-mcp` (MCP stdio) → DAW host bridge**. Talk in the official app. Do not paste Claude Pro/Max or ChatGPT Plus tokens into the plugin. The AU is not a chat client.

```
Claude Code / Codex / Cursor  (already logged in)
        |  MCP stdio
        v
     daw-mcp --host=logic
        |
        +--> Logic host bridge  (degraded: no project plugin API)
        +--> plugin IPC socket  (sidecar; L0 sensor in the AU)
```

Use a Release `daw-mcp`, a local `cargo build -p daw-mcp --release`, or the sidecar next to the plugin:

- `DAWAgent.clap/Contents/MacOS/daw-mcp`
- `DAWAgent.vst3/Contents/MacOS/daw-mcp`
- AU (installed): `~/Library/Audio/Plug-Ins/Components/DAWAgent.component/Contents/PlugIns/DAWAgent.clap/Contents/MacOS/daw-mcp`

The plugin GUI copies the Claude Code command and Codex toml from that path (buttons, no typing).

## Claude Code

```bash
claude mcp add daw-logic --transport stdio -- ~/Library/Audio/Plug-Ins/Components/DAWAgent.component/Contents/PlugIns/DAWAgent.clap/Contents/MacOS/daw-mcp --host=logic
```

Ableton (this wave):

```bash
claude mcp add daw-ableton --transport stdio -- /path/to/daw-mcp --host=ableton
```

## Codex

`~/.codex/config.toml`:

```toml
[mcp_servers.daw-logic]
command = "/Users/YOU/Library/Audio/Plug-Ins/Components/DAWAgent.component/Contents/PlugIns/DAWAgent.clap/Contents/MacOS/daw-mcp"
args = ["--host=logic"]
```

## Cursor

Settings → MCP → add a stdio server pointing at the same binary and `--host=logic` (or another host flag).

## Loopback HTTP (optional)

```bash
/path/to/daw-mcp --host=logic --http=8765
```

`--http=8765` still serves loopback HTTP; the plugin socket is always enabled unless you pass `--sidecar` (plugin IPC only, no stdio).

## Plugin role (not chat)

The VST3 / AU / CLAP editor talks to `daw-mcp` over a length-prefixed Unix socket for **status / health**:

- `~/Library/Application Support/DAW-Plugin-MCP/daw-mcp.sock`
- `/tmp/daw-mcp.sock`

The editor **spawns the bundled sidecar** (`daw-mcp --sidecar --host=<saved>`) if nothing is listening. First-time host defaults to **logic** and is remembered in `plugin-ui.json`. You do not need `cargo run`. Closing the plugin does not kill the sidecar.

Claude Code / Codex stdio starts **another** `daw-mcp` process for tools (`DawHost`). That path does **not** go through the AU text field (there isn’t one). Logic L3 writes are **unavailable** (`Logic has no project plugin API`). Health / local analysis still work. Next step: insert Probe, or use REAPER / Ableton.

Forbidden: `~/.claude` session files, `claude -p` as a hidden client, Claude.ai login in the plugin, using a subscription OAuth token as an API key.

Logic Pro: **音訊單元：Apple** is only Apple’s built-in AUs. File **DAW Agent** under Plug-in Manager **類別 → Utility**, then insert from **Utility** (search `daw` works after that). See [README.md](../README.md).

## Host flags

`reaper` `ableton` `bitwig` `logic` `studioone` `protools` `cubase` `flstudio`

## Plugin artifacts

Download VST3/AU from **Actions → Plugin artifacts**. Audio Units are macOS-only; Windows jobs publish VST3. The AU `.component` embeds `DAWAgent.clap` (with `daw-mcp` in `Contents/MacOS`); keep the standalone CLAP from the same `daw-plugin-macos-au` artifact if you also want a CLAP host to load it.
