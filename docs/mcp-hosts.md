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

## Plugin artifacts

Download VST3/AU from **Actions → Plugin artifacts**. Audio Units are macOS-only; Windows jobs publish VST3. The AU `.component` embeds `DAWAgent.clap`; keep the standalone CLAP from the same `daw-plugin-macos-au` artifact if you also want a CLAP host to load it.
