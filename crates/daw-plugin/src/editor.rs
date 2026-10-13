//! Custom egui editor shared by CLAP / VST3 / wrapped AU. Message thread only.

use crate::chat::{
    claude_mcp_add, codex_mcp_toml, fallback_sidecar_path, parse_capability_summary,
    parse_health_issues, ChatState, UiSettings, DAEMON_MISSING,
};
use crate::ipc;
use crate::sidecar;
use nih_plug::prelude::{Editor, ParamSetter};
use nih_plug_egui::egui::{self, Color32, CornerRadius, Frame, Margin, Stroke, Vec2};
use nih_plug_egui::{create_egui_editor, resizable_window::ResizableWindow, widgets, EguiState};
use parking_lot::Mutex;
use serde_json::json;
use std::sync::atomic::{AtomicU32, Ordering};
use std::sync::Arc;
use std::time::Duration;

const COL_BG: Color32 = Color32::from_rgb(18, 18, 22);
const COL_PANEL: Color32 = Color32::from_rgb(28, 28, 34);
const COL_OK: Color32 = Color32::from_rgb(86, 196, 122);
const COL_BAD: Color32 = Color32::from_rgb(214, 78, 78);
const COL_WARN: Color32 = Color32::from_rgb(214, 164, 72);
const COL_TEXT: Color32 = Color32::from_rgb(228, 228, 234);
const SIDEBAR_W: f32 = 260.0;

pub struct GuiInner {
    pub settings: UiSettings,
    pub chat: ChatState,
}

impl Default for GuiInner {
    fn default() -> Self {
        let mut chat = ChatState::new();
        chat.mcp_bin = sidecar::find_daemon_binary()
            .map(|p| p.display().to_string())
            .unwrap_or_else(|_| fallback_sidecar_path());
        Self {
            settings: UiSettings::load(),
            chat,
        }
    }
}

pub fn create_editor(
    editor_state: Arc<EguiState>,
    params: Arc<super::plugin::DawAgentParams>,
    peak: Arc<AtomicU32>,
) -> Option<Box<dyn Editor>> {
    let gui = Arc::new(Mutex::new(GuiInner::default()));
    let egui_state = editor_state.clone();
    create_egui_editor(
        editor_state,
        gui,
        |ctx, _| apply_theme(ctx),
        move |ctx, setter, gui| {
            ResizableWindow::new("daw-agent-ui")
                .min_size(Vec2::new(640.0, 420.0))
                .show(ctx, egui_state.as_ref(), |ui| {
                    draw(ui, ctx, setter, gui, &params, &peak);
                });
        },
    )
}

fn apply_theme(ctx: &egui::Context) {
    load_cjk_font(ctx);
    let mut v = egui::Visuals::dark();
    v.override_text_color = Some(COL_TEXT);
    v.window_fill = COL_BG;
    v.panel_fill = COL_PANEL;
    v.extreme_bg_color = COL_BG;
    v.window_corner_radius = CornerRadius::same(8);
    v.widgets.inactive.corner_radius = CornerRadius::same(6);
    v.widgets.hovered.corner_radius = CornerRadius::same(6);
    v.widgets.active.corner_radius = CornerRadius::same(6);
    v.widgets.inactive.bg_fill = COL_PANEL;
    v.selection.bg_fill = Color32::from_rgb(56, 92, 160);
    ctx.set_visuals(v);
}

fn load_cjk_font(ctx: &egui::Context) {
    const CANDIDATES: &[&str] = &[
        "/Library/Fonts/Arial Unicode.ttf",
        "/System/Library/Fonts/Supplemental/Arial Unicode.ttf",
        "/System/Library/Fonts/Hiragino Sans GB.ttc",
        "/System/Library/Fonts/STHeiti Light.ttc",
        "/System/Library/Fonts/PingFang.ttc",
    ];
    for path in CANDIDATES {
        let Ok(bytes) = std::fs::read(path) else {
            continue;
        };
        let mut fonts = egui::FontDefinitions::default();
        fonts.font_data.insert(
            "cjk".into(),
            std::sync::Arc::new(egui::FontData::from_owned(bytes)),
        );
        if let Some(fam) = fonts.families.get_mut(&egui::FontFamily::Proportional) {
            fam.insert(0, "cjk".into());
        }
        if let Some(fam) = fonts.families.get_mut(&egui::FontFamily::Monospace) {
            fam.push("cjk".into());
        }
        ctx.set_fonts(fonts);
        break;
    }
}

fn draw(
    ui: &mut egui::Ui,
    ctx: &egui::Context,
    setter: &ParamSetter,
    gui: &mut Arc<Mutex<GuiInner>>,
    params: &Arc<super::plugin::DawAgentParams>,
    peak: &Arc<AtomicU32>,
) {
    let mut refresh = false;
    let mut save_settings = false;
    let mut ping = false;
    let mut copy_claude = false;
    let mut copy_codex = false;

    {
        let mut g = gui.lock();
        if !g.chat.pinged {
            g.chat.pinged = true;
            ping = true;
        }

        let raw_peak = f32::from_bits(peak.load(Ordering::Relaxed));
        g.chat.displayed_peak = g.chat.displayed_peak.max(raw_peak) * 0.82 + raw_peak * 0.18;
        if g.chat.displayed_peak > 0.002 {
            ctx.request_repaint();
        }

        top_bar(ui, &g, &mut ping, &mut refresh);
        ui.add_space(8.0);

        let height = ui.available_height();
        ui.horizontal_top(|ui| {
            ui.set_min_height(height);
            ui.allocate_ui_with_layout(
                Vec2::new(SIDEBAR_W, height),
                egui::Layout::top_down(egui::Align::Min).with_cross_justify(true),
                |ui| sidebar(ui, setter, &mut g, params, &mut save_settings, &mut refresh),
            );
            ui.add_space(8.0);
            ui.separator();
            ui.add_space(8.0);
            ui.allocate_ui_with_layout(
                Vec2::new(ui.available_width(), height),
                egui::Layout::top_down(egui::Align::Min).with_cross_justify(true),
                |ui| mcp_pane(ui, &mut g, &mut copy_claude, &mut copy_codex),
            );
        });
    }

    if save_settings {
        let g = gui.lock();
        if let Err(e) = g.settings.save() {
            drop(g);
            gui.lock().chat.status = format!("儲存失敗：{e}");
        }
    }
    if ping {
        spawn_ping(gui.clone(), ctx.clone());
    }
    if refresh {
        spawn_refresh(gui.clone(), ctx.clone());
    }
    if copy_claude || copy_codex {
        let (bin, host) = {
            let g = gui.lock();
            (g.chat.mcp_bin.clone(), g.settings.host.clone())
        };
        if copy_claude {
            ctx.copy_text(claude_mcp_add(&bin, &host));
            gui.lock().chat.copy_note = "已複製 Claude Code 指令".into();
        }
        if copy_codex {
            ctx.copy_text(codex_mcp_toml(&bin, &host));
            gui.lock().chat.copy_note = "已複製 Codex config".into();
        }
    }
}

fn top_bar(ui: &mut egui::Ui, g: &GuiInner, ping: &mut bool, refresh: &mut bool) {
    ui.horizontal(|ui| {
        ui.heading("DAW Agent");
        ui.add_space(8.0);
        if g.chat.starting {
            pill(ui, "starting daw-mcp…", COL_WARN);
        } else {
            pill(
                ui,
                if g.chat.daemon_ok {
                    "daemon 已連線"
                } else {
                    DAEMON_MISSING
                },
                if g.chat.daemon_ok { COL_OK } else { COL_BAD },
            );
        }
        let host_label = if g.chat.host.is_empty() {
            g.settings.host.as_str()
        } else {
            g.chat.host.as_str()
        };
        if !host_label.is_empty() {
            pill(ui, host_label, Color32::from_rgb(90, 110, 150));
        }
        if g.chat.degraded {
            pill(ui, "degraded", COL_WARN);
        }
        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            if ui.button("Refresh").clicked() {
                *refresh = true;
            }
            if ui.button("Reconnect").clicked() {
                *ping = true;
            }
        });
    });
}

fn pill(ui: &mut egui::Ui, text: &str, fill: Color32) {
    Frame::new()
        .fill(fill.linear_multiply(0.28))
        .stroke(Stroke::new(1.0_f32, fill))
        .corner_radius(CornerRadius::same(10))
        .inner_margin(Margin::symmetric(8, 3))
        .show(ui, |ui| {
            ui.colored_label(fill, text);
        });
}

fn sidebar(
    ui: &mut egui::Ui,
    setter: &ParamSetter,
    g: &mut GuiInner,
    params: &Arc<super::plugin::DawAgentParams>,
    save_settings: &mut bool,
    refresh: &mut bool,
) {
    Frame::new()
        .fill(COL_PANEL)
        .corner_radius(CornerRadius::same(8))
        .inner_margin(Margin::same(10))
        .show(ui, |ui| {
            ui.label("Output");
            ui.add(widgets::ParamSlider::for_param(&params.output_gain, setter));
            let norm = peak_norm(g.chat.displayed_peak);
            ui.add(
                egui::ProgressBar::new(norm).text(if g.chat.displayed_peak < 1e-5 {
                    "-inf dB".into()
                } else {
                    format!("{:.1} dB", 20.0 * g.chat.displayed_peak.max(1e-8).log10())
                }),
            );

            ui.add_space(8.0);
            ui.horizontal(|ui| {
                ui.strong("Health");
                if ui.small_button("↻").clicked() {
                    *refresh = true;
                }
            });
            if g.chat.health_lines.is_empty() {
                ui.weak(if g.chat.daemon_ok {
                    "無 issue"
                } else {
                    DAEMON_MISSING
                });
            } else {
                for line in g.chat.health_lines.iter().take(3) {
                    ui.colored_label(COL_WARN, format!("{} · {}", line.code, line.message));
                }
            }
            if g.chat.degraded {
                ui.colored_label(
                    COL_WARN,
                    if g.chat.degraded_reason.is_empty() {
                        "capabilities degraded".to_string()
                    } else {
                        g.chat.degraded_reason.clone()
                    },
                );
            }

            ui.add_space(8.0);
            ui.strong("Host");
            ui.horizontal_wrapped(|ui| {
                for h in [
                    "logic",
                    "reaper",
                    "ableton",
                    "bitwig",
                    "studioone",
                    "protools",
                    "cubase",
                    "flstudio",
                ] {
                    ui.selectable_value(&mut g.settings.host, h.to_string(), h);
                }
            });
            if ui.button("儲存 Host").clicked() {
                *save_settings = true;
            }
            ui.weak("對話在 Claude Code / Codex，不在此外掛。");
        });
}

fn mcp_pane(ui: &mut egui::Ui, g: &mut GuiInner, copy_claude: &mut bool, copy_codex: &mut bool) {
    Frame::new()
        .fill(COL_BG)
        .corner_radius(CornerRadius::same(8))
        .inner_margin(Margin::same(12))
        .show(ui, |ui| {
            ui.heading("訂閱 MCP");
            ui.add_space(6.0);
            ui.label("在 Claude Code、Codex 或 Cursor 說話。此外掛不收 API key，也不讀訂閱 token。");
            ui.add_space(10.0);
            ui.strong("sidecar");
            ui.monospace(&g.chat.mcp_bin);
            if g.chat.starting {
                ui.colored_label(COL_WARN, "starting daw-mcp…");
            } else if !g.chat.daemon_ok {
                ui.colored_label(COL_BAD, &g.chat.status);
            }
            ui.add_space(10.0);
            if ui
                .add_sized(Vec2::new(ui.available_width(), 36.0), egui::Button::new("複製 Claude Code 指令"))
                .clicked()
            {
                *copy_claude = true;
            }
            ui.add_space(6.0);
            if ui
                .add_sized(Vec2::new(ui.available_width(), 36.0), egui::Button::new("複製 Codex config"))
                .clicked()
            {
                *copy_codex = true;
            }
            if !g.chat.copy_note.is_empty() {
                ui.add_space(6.0);
                ui.colored_label(COL_OK, &g.chat.copy_note);
            }
            ui.add_space(12.0);
            ui.weak("Cursor：Settings → MCP，stdio 指向同一 binary 與 --host。");
            ui.add_space(8.0);
            if g.chat.degraded {
                ui.colored_label(
                    COL_WARN,
                    if g.chat.degraded_reason.is_empty() {
                        "Logic 深層控制 degraded（無 project plugin API）。分析 / health 仍可用。".into()
                    } else {
                        format!(
                            "{} 下一步：Probe 或換 REAPER / Ableton。",
                            g.chat.degraded_reason
                        )
                    },
                );
            } else {
                ui.weak("Logic 深層控制為 degraded（無 project plugin API）。下一步：Probe 或換 REAPER / Ableton。");
            }
        });
}

fn peak_norm(peak: f32) -> f32 {
    if peak <= 1e-8 {
        0.0
    } else {
        ((20.0 * peak.log10() + 60.0) / 60.0).clamp(0.0, 1.0)
    }
}

fn apply_hello(gui: &Arc<Mutex<GuiInner>>, v: &serde_json::Value) {
    let host = v
        .get("host")
        .and_then(|h| h.as_str())
        .unwrap_or("unknown")
        .to_string();
    let mut g = gui.lock();
    g.chat.daemon_ok = true;
    g.chat.starting = false;
    g.chat.host = host.clone();
    g.chat.status = format!("daemon 已連線（host={host}）");
}

fn spawn_ping(gui: Arc<Mutex<GuiInner>>, ctx: egui::Context) {
    let host = gui.lock().settings.host.clone();
    std::thread::spawn(move || {
        match ipc::rpc_call("plugin/hello", json!({})) {
            Ok(v) => {
                apply_hello(&gui, &v);
                ctx.request_repaint();
                return;
            }
            Err(_) => {
                let mut g = gui.lock();
                g.chat.starting = true;
                g.chat.daemon_ok = false;
                g.chat.status = "starting daw-mcp…".into();
            }
        }
        ctx.request_repaint();
        match sidecar::spawn_if_needed(&host) {
            Ok(bin) => {
                {
                    let mut g = gui.lock();
                    g.chat.mcp_bin = bin.display().to_string();
                }
                if sidecar::wait_until_up(Duration::from_secs(5)) {
                    match ipc::rpc_call("plugin/hello", json!({})) {
                        Ok(v) => apply_hello(&gui, &v),
                        Err(e) => {
                            let mut g = gui.lock();
                            g.chat.starting = false;
                            g.chat.daemon_ok = false;
                            g.chat.status = e;
                        }
                    }
                } else {
                    let mut g = gui.lock();
                    g.chat.starting = false;
                    g.chat.daemon_ok = false;
                    g.chat.status = "starting daw-mcp… 逾時".into();
                }
            }
            Err(e) => {
                let mut g = gui.lock();
                g.chat.starting = false;
                g.chat.daemon_ok = false;
                g.chat.status = e;
            }
        }
        ctx.request_repaint();
    });
}

fn spawn_refresh(gui: Arc<Mutex<GuiInner>>, ctx: egui::Context) {
    std::thread::spawn(move || {
        let health_v = ipc::rpc_call(
            "tools/call",
            json!({"name": "daw_health_check", "arguments": {}}),
        );
        let cap_v = ipc::rpc_call(
            "tools/call",
            json!({"name": "daw_get_capabilities", "arguments": {}}),
        );
        let mut g = gui.lock();
        match health_v {
            Ok(v) => g.chat.health_lines = parse_health_issues(&v),
            Err(e) => {
                g.chat.health_lines = vec![crate::chat::HealthLine {
                    code: "ipc".into(),
                    message: e,
                }];
            }
        }
        if let Ok(v) = cap_v {
            let cap = parse_capability_summary(&v);
            if !cap.host.is_empty() {
                g.chat.host = cap.host;
            }
            g.chat.degraded = cap.degraded;
            g.chat.degraded_reason = cap.reason;
        }
        ctx.request_repaint();
    });
}

#[cfg(test)]
mod tests {
    use super::peak_norm;

    #[test]
    fn peak_norm_silence_is_zero() {
        assert_eq!(peak_norm(0.0), 0.0);
    }

    #[test]
    fn peak_norm_unity_is_one() {
        assert!((peak_norm(1.0) - 1.0).abs() < 0.001);
    }
}
