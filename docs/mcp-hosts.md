# Connect Claude Code, Cursor, or Codex

Subscription mode: this repo is an **MCP server**. Talk to the official CLI; do not paste Claude Pro/Max OAuth into the plugin.

```bash
cargo build -p daw-mcp
claude mcp add daw --transport stdio -- ./target/debug/daw-mcp --host=reaper
```

Ableton (this wave):

```bash
claude mcp add daw-ableton --transport stdio -- ./target/debug/daw-mcp --host=ableton
```

Loopback HTTP (127.0.0.1 only):

```bash
./target/debug/daw-mcp --host=reaper --http=8765
```

## Plugin chat / API mode

The VST3 / AU / CLAP editor talks to `daw-mcp` over a length-prefixed Unix socket (`$TMPDIR/daw-mcp.sock`) or Windows named pipe `\\.\pipe\daw-mcp`. Start the daemon first:

```bash
cargo run -p daw-mcp -- --host=logic
```

`--http=8765` still serves loopback HTTP; the plugin socket is always enabled.

Set **your** keys in the plugin Settings panel (or env):

- `ANTHROPIC_API_KEY`
- `OPENAI_API_KEY` / `OPENAI_BASE_URL`
- `OPENROUTER_API_KEY`
- or Ollama at `http://127.0.0.1:11434`

Keys entered in the UI are stored at `~/Library/Application Support/DAW-Plugin-MCP/plugin-ui.json` (mode `0600`). They are never logged.

Forbidden: `~/.claude` session files, `claude -p` as a hidden client, Claude.ai login in the plugin.

Logic Pro: the mixer search box does not list this AU. Use **Audio Units → DAW Plugin MCP → DAW Agent** on an Audio FX slot. See [README.md](../README.md).

## Host flags

`reaper` `ableton` `bitwig` `logic` `studioone` `protools` `cubase` `flstudio`

## Plugin artifacts

Download VST3/AU from **Actions → Plugin artifacts**. Audio Units are macOS-only; Windows jobs publish VST3. The AU `.component` embeds `DAWAgent.clap`; keep the standalone CLAP from the same `daw-plugin-macos-au` artifact if you also want a CLAP host to load it.
