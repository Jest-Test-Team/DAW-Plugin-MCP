//! Custom egui editor shared by CLAP / VST3 / wrapped AU. Message thread only.

use crate::chat::{
    chat_params, is_user, parse_capability_summary, parse_health_issues, take_draft, ChatState,
    UiSettings, DAEMON_MISSING,
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
const COL_USER: Color32 = Color32::from_rgb(42, 92, 168);
const COL_ASSIST: Color32 = Color32::from_rgb(44, 44, 52);
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
        Self {
            settings: UiSettings::load(),
            chat: ChatState::new(),
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
    let mut send_chat = false;
    let mut refresh = false;
    let mut save_settings = false;
    let mut ping = false;

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
                |ui| chat_pane(ui, &mut g, &mut send_chat),
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
    if send_chat {
        spawn_chat(gui.clone(), ctx.clone());
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
            ui.strong("設定");
            ui.horizontal_wrapped(|ui| {
                ui.selectable_value(&mut g.settings.kind, "ollama".into(), "Ollama");
                ui.selectable_value(&mut g.settings.kind, "anthropic_api".into(), "Anthropic");
                ui.selectable_value(&mut g.settings.kind, "openai_compat".into(), "OpenAI");
                ui.selectable_value(&mut g.settings.kind, "openrouter".into(), "OpenRouter");
            });
            ui.label("Base URL");
            ui.text_edit_singleline(&mut g.settings.base_url);
            ui.label("Model");
            ui.text_edit_singleline(&mut g.settings.model);
            ui.label("API key");
            ui.add(egui::TextEdit::singleline(&mut g.settings.api_key).password(true));
            ui.label("Host");
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
            if ui.button("儲存設定").clicked() {
                *save_settings = true;
            }
            ui.weak("金鑰只進本機設定。訂閱請用 Claude Code / Cursor 加 MCP。");
            if let Some(hint) = g.settings.provider_hint() {
                ui.colored_label(COL_WARN, hint);
            }
        });
}

fn chat_pane(ui: &mut egui::Ui, g: &mut GuiInner, send_chat: &mut bool) {
    let composer_h = 96.0;
    let hist_h = (ui.available_height() - composer_h).max(80.0);

    Frame::new()
        .fill(COL_BG)
        .corner_radius(CornerRadius::same(8))
        .inner_margin(Margin::same(8))
        .show(ui, |ui| {
            egui::ScrollArea::vertical()
                .max_height(hist_h)
                .stick_to_bottom(true)
                .show(ui, |ui| {
                    if g.chat.messages.is_empty() && !g.chat.busy {
                        if g.chat.starting {
                            ui.weak("starting daw-mcp…");
                        } else if !g.chat.daemon_ok {
                            ui.vertical_centered(|ui| {
                                ui.add_space(40.0);
                                ui.colored_label(COL_BAD, &g.chat.status);
                            });
                        }
                    }
                    for m in &g.chat.messages {
                        bubble(ui, is_user(&m.role), &m.text);
                    }
                    if g.chat.busy {
                        bubble(ui, false, "…");
                    }
                });

            ui.add_space(6.0);
            ui.horizontal(|ui| {
                let enabled = !g.chat.busy && g.chat.daemon_ok;
                let edit = egui::TextEdit::multiline(&mut g.chat.draft)
                    .desired_rows(2)
                    .desired_width(ui.available_width() - 88.0)
                    .hint_text("問 session / health…");
                let resp = ui.add_enabled(enabled, edit);
                let clicked = ui
                    .add_enabled(
                        enabled,
                        egui::Button::new("送出").min_size(Vec2::new(80.0, 48.0)),
                    )
                    .clicked();
                let mut enter = false;
                if enabled && resp.has_focus() {
                    ui.input_mut(|i| {
                        i.events.retain(|ev| match ev {
                            egui::Event::Key {
                                key: egui::Key::Enter,
                                pressed: true,
                                modifiers,
                                ..
                            } if !modifiers.shift => {
                                enter = true;
                                false
                            }
                            _ => true,
                        });
                    });
                }
                if clicked || enter {
                    *send_chat = true;
                }
            });
            if let Some(hint) = g.settings.provider_hint() {
                ui.colored_label(COL_WARN, hint);
            }
        });
}

fn bubble(ui: &mut egui::Ui, user: bool, text: &str) {
    ui.horizontal(|ui| {
        if user {
            let spacer = (ui.available_width() * 0.18).max(12.0);
            ui.add_space(spacer);
        }
        let fill = if user { COL_USER } else { COL_ASSIST };
        Frame::new()
            .fill(fill)
            .corner_radius(CornerRadius::same(10))
            .inner_margin(Margin::symmetric(10, 8))
            .show(ui, |ui| {
                ui.set_max_width(ui.available_width() * 0.92);
                ui.label(text);
            });
    });
    ui.add_space(4.0);
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
            Ok(_) => {
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

fn spawn_chat(gui: Arc<Mutex<GuiInner>>, ctx: egui::Context) {
    let params = {
        let mut g = gui.lock();
        let Some(draft) = take_draft(&g.chat.draft) else {
            return;
        };
        if g.chat.busy || !g.chat.daemon_ok {
            return;
        }
        g.chat.draft.clear();
        g.chat.push_user(draft);
        if let Some(hint) = g.settings.provider_hint() {
            g.chat.push_assistant(hint.to_string());
            None
        } else {
            g.chat.busy = true;
            Some(chat_params(&g.chat.messages, &g.settings))
        }
    };
    ctx.request_repaint();
    let Some(params) = params else {
        return;
    };
    std::thread::spawn(move || {
        let result = ipc::rpc_call("assistant/chat", params);
        let mut g = gui.lock();
        match result {
            Ok(v) => {
                let text = v
                    .get("text")
                    .and_then(|t| t.as_str())
                    .unwrap_or("")
                    .to_string();
                if text.is_empty() {
                    g.chat.push_assistant("錯誤：空回應".into());
                } else {
                    g.chat.push_assistant(text);
                }
            }
            Err(e) => g.chat.push_assistant(format!("錯誤：{e}")),
        }
        g.chat.busy = false;
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
