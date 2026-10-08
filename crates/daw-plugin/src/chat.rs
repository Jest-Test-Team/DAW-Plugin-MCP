//! Plugin-UI chat/settings. Message thread only — never from `process()`.

use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::fs;
use std::io::Write;
use std::path::PathBuf;

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct ChatMessage {
    pub role: String,
    pub text: String,
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct HealthLine {
    pub code: String,
    pub message: String,
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct CapSummary {
    pub host: String,
    pub degraded: bool,
    pub reason: String,
}

#[derive(Clone, Debug, Default)]
pub struct ChatState {
    pub messages: Vec<ChatMessage>,
    pub draft: String,
    pub busy: bool,
    pub daemon_ok: bool,
    pub status: String,
    pub host: String,
    pub degraded: bool,
    pub degraded_reason: String,
    pub health_lines: Vec<HealthLine>,
    pub pinged: bool,
    pub displayed_peak: f32,
}

impl ChatState {
    pub fn new() -> Self {
        Self {
            status: "請啟動 daw-mcp".into(),
            ..Self::default()
        }
    }

    pub fn push_user(&mut self, text: String) {
        self.messages.push(ChatMessage {
            role: "user".into(),
            text,
        });
    }

    pub fn push_assistant(&mut self, text: String) {
        self.messages.push(ChatMessage {
            role: "assistant".into(),
            text,
        });
    }
}

#[derive(Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct UiSettings {
    pub kind: String,
    pub api_key: String,
    pub model: String,
    pub base_url: String,
}

impl std::fmt::Debug for UiSettings {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("UiSettings")
            .field("kind", &self.kind)
            .field("api_key", &redact(&self.api_key))
            .field("model", &self.model)
            .field("base_url", &self.base_url)
            .finish()
    }
}

fn redact(key: &str) -> String {
    if key.is_empty() {
        String::new()
    } else {
        "***".into()
    }
}

impl Default for UiSettings {
    fn default() -> Self {
        Self {
            kind: "ollama".into(),
            api_key: String::new(),
            model: "llama3.2".into(),
            base_url: "http://127.0.0.1:11434".into(),
        }
    }
}

impl UiSettings {
    pub fn settings_path() -> PathBuf {
        if let Ok(p) = std::env::var("DAW_PLUGIN_UI_CONFIG") {
            return PathBuf::from(p);
        }
        let home = std::env::var("HOME")
            .or_else(|_| std::env::var("USERPROFILE"))
            .unwrap_or_else(|_| ".".into());
        let home = PathBuf::from(home);
        if cfg!(target_os = "macos") {
            home.join("Library/Application Support/DAW-Plugin-MCP/plugin-ui.json")
        } else if cfg!(windows) {
            home.join("AppData/Roaming/DAW-Plugin-MCP/plugin-ui.json")
        } else {
            home.join(".config/DAW-Plugin-MCP/plugin-ui.json")
        }
    }

    pub fn load() -> Self {
        let path = Self::settings_path();
        let Ok(raw) = fs::read_to_string(&path) else {
            return Self::default();
        };
        serde_json::from_str(&raw).unwrap_or_default()
    }

    pub fn save(&self) -> Result<(), String> {
        let path = Self::settings_path();
        if let Some(dir) = path.parent() {
            fs::create_dir_all(dir).map_err(|e| e.to_string())?;
        }
        let body = serde_json::to_vec_pretty(self).map_err(|e| e.to_string())?;
        let mut f = fs::OpenOptions::new()
            .create(true)
            .write(true)
            .truncate(true)
            .open(&path)
            .map_err(|e| e.to_string())?;
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let _ = f.set_permissions(fs::Permissions::from_mode(0o600));
        }
        f.write_all(&body).map_err(|e| e.to_string())?;
        Ok(())
    }

    pub fn to_provider_value(&self) -> Value {
        match self.kind.as_str() {
            "anthropic_api" => json!({
                "kind": "anthropic_api",
                "api_key": self.api_key,
                "base_url": nonempty(&self.base_url, "https://api.anthropic.com"),
                "model": nonempty(&self.model, "claude-sonnet-4-20250514")
            }),
            "openai_compat" => json!({
                "kind": "openai_compat",
                "api_key": self.api_key,
                "base_url": nonempty(&self.base_url, "https://api.openai.com/v1"),
                "model": nonempty(&self.model, "gpt-4.1")
            }),
            "openrouter" => json!({
                "kind": "openrouter",
                "api_key": self.api_key,
                "base_url": nonempty(&self.base_url, "https://openrouter.ai/api/v1"),
                "model": nonempty(&self.model, "anthropic/claude-sonnet-4")
            }),
            _ => json!({
                "kind": "ollama",
                "base_url": nonempty(&self.base_url, "http://127.0.0.1:11434"),
                "model": nonempty(&self.model, "llama3.2")
            }),
        }
    }
}

fn nonempty<'a>(s: &'a str, fallback: &'a str) -> &'a str {
    if s.trim().is_empty() {
        fallback
    } else {
        s
    }
}

pub fn chat_params(messages: &[ChatMessage], settings: &UiSettings) -> Value {
    json!({
        "messages": messages.iter().map(|m| json!({
            "role": m.role,
            "content": m.text
        })).collect::<Vec<_>>(),
        "provider": settings.to_provider_value()
    })
}

pub fn jsonrpc_request(id: u64, method: &str, params: Value) -> Value {
    json!({
        "jsonrpc": "2.0",
        "id": id,
        "method": method,
        "params": params
    })
}

pub const DAEMON_MISSING: &str = "請啟動 daw-mcp";

pub fn is_user(role: &str) -> bool {
    role.eq_ignore_ascii_case("user")
}

fn structured(v: &Value) -> &Value {
    v.get("structuredContent").unwrap_or(v)
}

/// Pull up to three health issues from a `tools/call` result. No JSON dump.
pub fn parse_health_issues(rpc_result: &Value) -> Vec<HealthLine> {
    let sc = structured(rpc_result);
    let Some(arr) = sc.as_array() else {
        return Vec::new();
    };
    arr.iter()
        .filter_map(|item| {
            let code = item.get("code").and_then(|c| c.as_str())?;
            let message = item.get("message").and_then(|m| m.as_str())?;
            Some(HealthLine {
                code: code.to_string(),
                message: message.to_string(),
            })
        })
        .take(3)
        .collect()
}

pub fn parse_capability_summary(rpc_result: &Value) -> CapSummary {
    let sc = structured(rpc_result);
    let host = sc
        .get("host")
        .and_then(|h| h.as_str())
        .unwrap_or("unknown")
        .to_string();
    let mut degraded = false;
    let mut reason = String::new();
    if let Some(arr) = sc.get("capabilities").and_then(|c| c.as_array()) {
        for cap in arr {
            let support = cap.get("support").and_then(|s| s.as_str()).unwrap_or("");
            if support == "degraded" || support == "unavailable" {
                degraded = true;
                if reason.is_empty() {
                    reason = cap
                        .get("reason")
                        .and_then(|r| r.as_str())
                        .unwrap_or("host capabilities limited")
                        .to_string();
                }
            }
        }
    }
    CapSummary {
        host,
        degraded,
        reason,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::{SystemTime, UNIX_EPOCH};

    #[test]
    fn push_user_then_assistant() {
        let mut c = ChatState::new();
        c.push_user("hello".into());
        c.push_assistant("hi".into());
        assert_eq!(c.messages.len(), 2);
        assert_eq!(c.messages[0].role, "user");
        assert_eq!(c.status, DAEMON_MISSING);
    }

    #[test]
    fn settings_debug_redacts_key() {
        let s = UiSettings {
            kind: "anthropic_api".into(),
            api_key: "sk-secret-value".into(),
            model: "claude".into(),
            base_url: String::new(),
        };
        let d = format!("{s:?}");
        assert!(!d.contains("sk-secret-value"));
        assert!(d.contains("***"));
    }

    #[test]
    fn settings_roundtrip_via_env_path() {
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let path = std::env::temp_dir().join(format!("daw-plugin-ui-{nanos}.json"));
        std::env::set_var("DAW_PLUGIN_UI_CONFIG", &path);
        let s = UiSettings {
            kind: "openrouter".into(),
            api_key: "k".into(),
            model: "m".into(),
            base_url: "https://openrouter.ai/api/v1".into(),
        };
        s.save().unwrap();
        let loaded = UiSettings::load();
        assert_eq!(loaded.kind, "openrouter");
        assert_eq!(loaded.api_key, "k");
        let _ = fs::remove_file(&path);
        std::env::remove_var("DAW_PLUGIN_UI_CONFIG");
    }

    #[test]
    fn chat_params_include_provider_kind() {
        let mut c = ChatState::new();
        c.push_user("ping".into());
        let p = chat_params(&c.messages, &UiSettings::default());
        assert_eq!(p["messages"][0]["content"], "ping");
        assert_eq!(p["provider"]["kind"], "ollama");
        let req = jsonrpc_request(1, "assistant/chat", p);
        assert_eq!(req["method"], "assistant/chat");
        assert_eq!(req["id"], 1);
    }

    #[test]
    fn is_user_matches_role() {
        assert!(is_user("user"));
        assert!(is_user("User"));
        assert!(!is_user("assistant"));
    }

    #[test]
    fn parse_health_from_structured_content() {
        let v = json!({
            "content": [{"type":"text","text":"```json\n[]\n```"}],
            "structuredContent": [
                {"code":"clip","message":"track 1 clipping","severity":"warn"},
                {"code":"masking","message":"overlap 250 Hz","severity":"warn"},
                {"code":"unnamed","message":"untitled track","severity":"info"},
                {"code":"extra","message":"should drop","severity":"info"}
            ]
        });
        let lines = parse_health_issues(&v);
        assert_eq!(lines.len(), 3);
        assert_eq!(lines[0].code, "clip");
        assert!(!serde_json::to_string(&v).unwrap().is_empty());
    }

    #[test]
    fn parse_capability_marks_degraded_logic() {
        let v = json!({
            "structuredContent": {
                "host": "logic",
                "layers": ["l2_surface"],
                "capabilities": [
                    {"tool":"daw_get_capabilities","support":"supported"},
                    {
                        "tool":"daw_set_parameter",
                        "support":"unavailable",
                        "reason":"Logic has no project plugin API"
                    }
                ]
            }
        });
        let s = parse_capability_summary(&v);
        assert_eq!(s.host, "logic");
        assert!(s.degraded);
        assert!(s.reason.contains("no project plugin API"));
    }
}
