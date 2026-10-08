# DAW Agent MCP

Rust MCP daemon + VST3/CLAP/AU plugin shell. Spectral analysis in **Julia**. Tests in **Robot Framework**.

## Quick start

```bash
cargo test --workspace
cargo run -p daw-mcp -- --host=reaper
```

Ableton is included in this wave (`--host=ableton`).

Subscription users add the binary as an MCP server in Claude Code / Cursor / Codex. See [docs/mcp-hosts.md](docs/mcp-hosts.md).

## Plugin bundles (VST3 / AU)

AU can only be built on macOS. Locally:

```bash
cargo xtask bundle daw-plugin --release
# macOS AU (requires cmake + clap-wrapper clone):
bash scripts/wrap_au.sh
```

CI uploads GitHub Actions artifacts from [`.github/workflows/plugin.yml`](.github/workflows/plugin.yml). Open **Actions → Plugin artifacts →** the run you want **→ Artifacts**:

- `daw-plugin-macos-vst3`
- `daw-plugin-macos-au` (`.component` with embedded CLAP, plus `DAWAgent.clap`)
- `daw-plugin-windows-vst3`

AU is macOS-only; the Windows job does not build Audio Units. Ad-hoc codesign only — Developer ID is not in CI. See [docs/ci-security.md](docs/ci-security.md) for SAST / DAST / IAST.

Tagged releases (`git tag v0.1.0 && git push origin v0.1.0`) run [`.github/workflows/release.yml`](.github/workflows/release.yml), zip the macOS/Windows bundles, and attach them to a GitHub Release.

The Vercel production URL is a **static landing page** (`public/`). It does not run `daw-mcp` or Python.
