//! Custom egui editor shared by CLAP / VST3 / wrapped AU. Message thread only.

use crate::chat::{chat_params, ChatState, UiSettings, DAEMON_MISSING};
use crate::ipc;
use nih_plug::prelude::{Editor, ParamSetter};
use nih_plug_egui::{create_egui_editor, egui, widgets, EguiState};
use parking_lot::Mutex;
use serde_json::json;
use std::sync::Arc;

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
) -> Option<Box<dyn Editor>> {
    let gui = Arc::new(Mutex::new(GuiInner::default()));
    create_egui_editor(
        editor_state,
        gui,
        |_, _| {},
        move |ctx, setter, gui| {
            draw(ctx, setter, gui, &params);
        },
    )
}

fn draw(
    ctx: &egui::Context,
    setter: &ParamSetter,
    gui: &mut Arc<Mutex<GuiInner>>,
    params: &Arc<super::plugin::DawAgentParams>,
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

        egui::CentralPanel::default().show(ctx, |ui| {
            ui.heading("DAW Agent");
            ui.label("外掛內建對話只走你自己的 API key / OpenRouter / Ollama。訂閱請用 Claude Code 或 Cursor 加 MCP，不要貼 Pro/Max 登入。");
            ui.horizontal(|ui| {
                let color = if g.chat.daemon_ok {
                    egui::Color32::from_rgb(80, 180, 80)
                } else {
                    egui::Color32::from_rgb(200, 80, 80)
                };
                ui.colored_label(color, &g.chat.status);
                if ui.button("重新連線").clicked() {
                    ping = true;
                }
                if ui.button("重新整理 health / session").clicked() {
                    refresh = true;
                }
            });
            ui.add(widgets::ParamSlider::for_param(&params.output_gain, setter));
            ui.separator();

            egui::ScrollArea::vertical()
                .max_height(180.0)
                .show(ui, |ui| {
                    ui.collapsing("Health / session", |ui| {
                        ui.label(&g.chat.health);
                        ui.separator();
                        ui.label(&g.chat.session);
                    });
                    ui.collapsing("設定", |ui| {
                        ui.label("Provider");
                        ui.horizontal(|ui| {
                            ui.selectable_value(&mut g.settings.kind, "ollama".into(), "Ollama");
                            ui.selectable_value(
                                &mut g.settings.kind,
                                "anthropic_api".into(),
                                "Anthropic API",
                            );
                            ui.selectable_value(
                                &mut g.settings.kind,
                                "openai_compat".into(),
                                "OpenAI-compat",
                            );
                            ui.selectable_value(
                                &mut g.settings.kind,
                                "openrouter".into(),
                                "OpenRouter",
                            );
                        });
                        ui.label("Base URL");
                        ui.text_edit_singleline(&mut g.settings.base_url);
                        ui.label("Model");
                        ui.text_edit_singleline(&mut g.settings.model);
                        ui.label("API key（不進 log、不進音訊執行緒）");
                        ui.add(
                            egui::TextEdit::singleline(&mut g.settings.api_key).password(true),
                        );
                        if ui.button("儲存設定").clicked() {
                            save_settings = true;
                        }
                    });
                });

            ui.separator();
            ui.label("對話");
            egui::ScrollArea::vertical()
                .max_height(160.0)
                .stick_to_bottom(true)
                .show(ui, |ui| {
                    for m in &g.chat.messages {
                        ui.label(format!("{}: {}", m.role, m.text));
                    }
                    if g.chat.busy {
                        ui.label("…");
                    }
                });
            ui.horizontal(|ui| {
                let enabled = !g.chat.busy && g.chat.daemon_ok;
                let edit = egui::TextEdit::singleline(&mut g.chat.draft)
                    .desired_width(480.0)
                    .hint_text("問 session / health…");
                let resp = ui.add_enabled(enabled, edit);
                let clicked = ui
                    .add_enabled(enabled, egui::Button::new("送出"))
                    .clicked();
                if clicked || (resp.lost_focus() && ui.input(|i| i.key_pressed(egui::Key::Enter)))
                {
                    send_chat = true;
                }
            });
            if !g.chat.daemon_ok {
                ui.weak(DAEMON_MISSING);
            }
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

fn spawn_ping(gui: Arc<Mutex<GuiInner>>, ctx: egui::Context) {
    std::thread::spawn(move || {
        match ipc::rpc_call("plugin/hello", json!({})) {
            Ok(v) => {
                let host = v.get("host").and_then(|h| h.as_str()).unwrap_or("unknown");
                let mut g = gui.lock();
                g.chat.daemon_ok = true;
                g.chat.status = format!("daemon 已連線（host={host}）");
            }
            Err(e) => {
                let mut g = gui.lock();
                g.chat.daemon_ok = false;
                g.chat.status = e;
            }
        }
        ctx.request_repaint();
    });
}

fn spawn_refresh(gui: Arc<Mutex<GuiInner>>, ctx: egui::Context) {
    std::thread::spawn(move || {
        let health = tool_text("daw_health_check");
        let session = tool_text("daw_get_session_summary");
        let mut g = gui.lock();
        g.chat.health = health;
        g.chat.session = session;
        ctx.request_repaint();
    });
}

fn tool_text(name: &str) -> String {
    match ipc::rpc_call("tools/call", json!({"name": name, "arguments": {}})) {
        Ok(v) => v
            .pointer("/content/0/text")
            .and_then(|t| t.as_str())
            .map(str::to_string)
            .unwrap_or_else(|| v.to_string()),
        Err(e) => e,
    }
}

fn spawn_chat(gui: Arc<Mutex<GuiInner>>, ctx: egui::Context) {
    let params = {
        let mut g = gui.lock();
        let draft = g.chat.draft.trim().to_string();
        if draft.is_empty() || g.chat.busy || !g.chat.daemon_ok {
            return;
        }
        g.chat.draft.clear();
        g.chat.push_user(draft);
        g.chat.busy = true;
        chat_params(&g.chat.messages, &g.settings)
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
                g.chat.push_assistant(text);
            }
            Err(e) => g.chat.push_assistant(format!("錯誤：{e}")),
        }
        g.chat.busy = false;
        ctx.request_repaint();
    });
}
