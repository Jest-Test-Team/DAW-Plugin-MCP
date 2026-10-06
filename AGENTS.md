# File ownership (13 sub-agents + main)

Main language: **Rust**. Spectral / loudness / masking analysis: **Julia** in `analysis/`.
Tests owned by main agent: `tests/robot/`. Do not edit that tree.

Do **not** rewrite workspace `Cargo.toml` members. You may add dependencies only inside your crate's `Cargo.toml`.
Do **not** intercept `~/.claude` / `~/.cursor` tokens. Do **not** wrap `claude -p` as a hidden chat client.
Audio/realtime paths: no alloc, no filesystem, no network, no mutex on the audio callback.

| Agent | Owns exclusively |
|---|---|
| contracts | `crates/daw-contracts/`, `contracts/` |
| daemon-mcp | `crates/daw-mcp/` |
| plugin-l0 | `crates/daw-plugin/` |
| transaction | `crates/daw-transaction/` |
| bridge-reaper | `crates/daw-bridge-reaper/`, `bridges/reaper/` |
| bridge-ableton | `crates/daw-bridge-ableton/`, `bridges/ableton/` |
| bridge-bitwig | `crates/daw-bridge-bitwig/`, `bridges/bitwig/` |
| probe-l1 | `crates/daw-probe/` |
| bridge-logic | `crates/daw-bridge-logic/`, `bridges/logic/` |
| bridge-studioone | `crates/daw-bridge-studioone/`, `bridges/studioone/` |
| bridge-protools | `crates/daw-bridge-protools/`, `bridges/protools/` |
| bridges-p3 | `crates/daw-bridge-cubase/`, `crates/daw-bridge-flstudio/`, `bridges/cubase/`, `bridges/flstudio/` |
| eval-docs | `eval/`, `docs/` |
| main | `tests/robot/`, `analysis/`, root `Cargo.toml`, `README.md` |

Shared API: depend on `daw-contracts` (`DawHost` trait + serde types). Each bridge crate implements `DawHost` and exposes `fn connect() -> Result<Box<dyn DawHost>>` plus a `mock()` backend for tests.

IPC: length-prefixed JSON-RPC over Unix domain socket (macOS/Linux) / named pipe (Windows). Schema lives in `contracts/ipc.json`.
