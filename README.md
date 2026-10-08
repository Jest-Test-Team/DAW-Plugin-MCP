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

Quit Logic completely, reopen it, then **Plug-in Manager → Reset & Rescan Selection** (or Full Audio Unit Reset).

**Do not open 音訊單元：Apple.** That submenu is only Apple’s built-in AUs (AUDelay, AUBandpass, …). DAW Agent is not there, and mixer search will not find it until you file it in a Logic category:

1. Logic → Plug-in Manager
2. Left sidebar **類別 → Utility** (or click **+** on 類別 if you want a custom collection)
3. From **製造商 → DAW Plugin MCP**, drag **DAW Agent** onto **Utility**
4. Click **Done**

Then on an audio track Audio FX slot open **Utility → DAW Agent**. Search `daw` works after this assignment. It is an effect (`aufx`), not an instrument.

### Plugin GUI (AU / VST3 / CLAP)

The editor is a custom chat console (not Logic’s generic parameter view). Start the daemon **before** opening the plugin window:

```bash
cargo run -p daw-mcp -- --host=logic
```

If the editor says **請啟動 daw-mcp**, the Unix socket (`$TMPDIR/daw-mcp.sock`) is not up. Chat uses API keys / OpenRouter / Ollama from the Settings panel (saved to `~/Library/Application Support/DAW-Plugin-MCP/plugin-ui.json`). Subscription users should add MCP in Claude Code / Cursor instead of pasting Pro/Max login into the plugin.

After a new GitHub Actions build, replace the `.component` (AU) or `.vst3` and run `xattr -cr` again so Logic loads the GUI binary.

### Optional privacy / Gatekeeper

CI artifacts are **ad-hoc signed** (no Developer ID). After a GitHub download, macOS may quarantine the bundle.

1. `xattr -cr` (above) removes the quarantine flag.
2. If System Settings → **Privacy & Security** shows that DAWAgent was blocked, choose **Open Anyway**.
3. Confirm the signature if validation still fails:

```bash
codesign --verify --verbose=4 ~/Library/Audio/Plug-Ins/Components/DAWAgent.component
```


<img width="788" height="700" alt="截圖 2026-10-08 21 57 50" src="https://github.com/user-attachments/assets/8a27c90f-9b9b-4b29-8ec3-bc5090a7cb9a" />
