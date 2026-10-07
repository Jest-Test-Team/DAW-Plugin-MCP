# CI security suite (SAST / DAST / IAST)

GitHub Actions runs three layers. None of them scan `~/.claude` or subscription session files.

| Layer | Tools | What it covers |
| --- | --- | --- |
| **SAST** | CodeQL (Rust + Python), `clippy -D warnings`, `rustfmt` | Daemon, plugin, bridges, Robot/Python |
| **DAST** | OWASP ZAP baseline | `daw-mcp --http` on `127.0.0.1` only |
| **IAST substitute** | `cargo-fuzz` + Robot `security.robot` | IPC/`JSON-RPC` parsers and a live daemon |

Rust has no free in-process IAST agent (Contrast/Invicti). Fuzz + Robot is the substitute.

## SAST (static)

Workflow: [`.github/workflows/sast.yml`](../.github/workflows/sast.yml)

- `cargo fmt --check`
- `cargo clippy --workspace --all-targets -- -D warnings`
- GitHub CodeQL for **Rust** and **Python** (bridges / Robot)

This is compile-time analysis of the daemon, plugin, and DAW bridges.

## DAST (dynamic)

Workflow: [`.github/workflows/dast-iast.yml`](../.github/workflows/dast-iast.yml) job `dast`

The only HTTP surface is `daw-mcp --http`, which **binds `127.0.0.1` only**. ZAP baseline crawls `http://127.0.0.1:8765` (health + JSON-RPC). Informational ZAP findings do not fail the job; **High** risk does.

ZAP is **not** attacking the realtime audio callback. DSP has no sockets.

## IAST (interactive substitute)

Rust has no free in-process IAST agent comparable to Contrast/Invicti. CI substitutes:

- `cargo fuzz` on `decode_frame` (plugin IPC) and `parse_jsonrpc` (MCP bodies)
- Robot [`tests/robot/security.robot`](../tests/robot/security.robot): loopback bind, Logic writes stay `isError`, responses must not include the runner home directory

## Plugin artifacts

Workflow: [`.github/workflows/plugin.yml`](../.github/workflows/plugin.yml)

- **macOS**: VST3 + CLAP via `cargo xtask bundle`, AU via clap-wrapper (`scripts/wrap_au.sh`, `aufx` with embedded CLAP), ad-hoc `codesign`
- **Windows**: VST3 only (Audio Units do not exist on Windows)
- Apple **Developer ID** signing is not configured; hosts may prompt on unsigned CI builds
