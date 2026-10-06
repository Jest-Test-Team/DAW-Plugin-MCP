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

Set **your** keys only:

- `ANTHROPIC_API_KEY`
- `OPENAI_API_KEY` / `OPENAI_BASE_URL`
- `OPENROUTER_API_KEY`
- or Ollama at `http://127.0.0.1:11434`

Forbidden: `~/.claude` session files, `claude -p` as a hidden client, Claude.ai login in the plugin.

## Host flags

`reaper` `ableton` `bitwig` `logic` `studioone` `protools` `cubase` `flstudio`
