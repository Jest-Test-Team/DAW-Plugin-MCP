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

## Install AU in Logic Pro

Logic Pro loads **Audio Units only**. Copy `DAWAgent.component` from the `daw-plugin-macos-au` artifact; do not install `DAWAgent.clap` into Logic (CLAP is for other hosts). Keep the command on **one line** — a trailing `\` is line continuation and will make `cp` fail with `No such file or directory`.

```bash
mkdir -p ~/Library/Audio/Plug-Ins/Components
cp -R ~/Downloads/daw-plugin-macos-au/DAWAgent.component ~/Library/Audio/Plug-Ins/Components/
xattr -cr ~/Library/Audio/Plug-Ins/Components/DAWAgent.component
```

Quit Logic completely, reopen it, then **Plug-in Manager → Reset & Rescan Selection** (or Full Audio Unit Reset). Search for **DAW Agent** (manufacturer **DAW Plugin MCP**, type **Effect**). Insert it on an audio track **Audio FX** slot — it is `aufx`, not an instrument.

### Optional privacy / Gatekeeper

CI artifacts are **ad-hoc signed** (no Developer ID). After a GitHub download, macOS may quarantine the bundle.

1. `xattr -cr` (above) removes the quarantine flag.
2. If System Settings → **Privacy & Security** shows that DAWAgent was blocked, choose **Open Anyway**.
3. Confirm the signature if validation still fails:

```bash
codesign --verify --verbose=4 ~/Library/Audio/Plug-Ins/Components/DAWAgent.component
```