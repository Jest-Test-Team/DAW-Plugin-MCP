//! Official API / OpenRouter / Ollama only. Never read ~/.claude or wrap `claude -p`.

use anyhow::Context;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::sync::OnceLock;
use std::time::Duration;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum LlmProvider {
    AnthropicApi {
        api_key: String,
        #[serde(default = "anthropic_base")]
        base_url: String,
        #[serde(default = "anthropic_model")]
        model: String,
    },
    OpenAiCompat {
        api_key: String,
        base_url: String,
        model: String,
    },
    OpenRouter {
        api_key: String,
        #[serde(default = "openrouter_base")]
        base_url: String,
        model: String,
    },
    Ollama {
        #[serde(default = "ollama_base")]
        base_url: String,
        model: String,
    },
}

fn anthropic_base() -> String {
    "https://api.anthropic.com".into()
}
fn anthropic_model() -> String {
    "claude-sonnet-4-20250514".into()
}
fn openrouter_base() -> String {
    "https://openrouter.ai/api/v1".into()
}
fn ollama_base() -> String {
    "http://127.0.0.1:11434".into()
}

/// URL + which auth header is used. Never includes secrets.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ChatHttpPlan {
    pub url: String,
    /// `x-api-key`, `authorization`, or none for Ollama.
    pub auth: Option<&'static str>,
}

pub fn plan_chat_request(provider: &LlmProvider) -> ChatHttpPlan {
    match provider {
        LlmProvider::AnthropicApi { base_url, .. } => ChatHttpPlan {
            url: format!("{}/v1/messages", trim_slash(base_url)),
            auth: Some("x-api-key"),
        },
        LlmProvider::OpenAiCompat { base_url, .. } => ChatHttpPlan {
            url: format!("{}/chat/completions", trim_slash(base_url)),
            auth: Some("authorization"),
        },
        LlmProvider::OpenRouter { base_url, .. } => ChatHttpPlan {
            url: format!("{}/chat/completions", trim_slash(base_url)),
            auth: Some("authorization"),
        },
        LlmProvider::Ollama { base_url, .. } => ChatHttpPlan {
            url: format!("{}/api/chat", trim_slash(base_url)),
            auth: None,
        },
    }
}

fn trim_slash(url: &str) -> &str {
    url.trim_end_matches('/')
}

impl LlmProvider {
    pub fn from_env() -> Option<Self> {
        if let Ok(key) = std::env::var("ANTHROPIC_API_KEY") {
            return Some(Self::AnthropicApi {
                api_key: key,
                base_url: std::env::var("ANTHROPIC_BASE_URL").unwrap_or_else(|_| anthropic_base()),
                model: std::env::var("ANTHROPIC_MODEL").unwrap_or_else(|_| anthropic_model()),
            });
        }
        if let Ok(key) = std::env::var("OPENROUTER_API_KEY") {
            return Some(Self::OpenRouter {
                api_key: key,
                base_url: openrouter_base(),
                model: std::env::var("OPENROUTER_MODEL")
                    .unwrap_or_else(|_| "anthropic/claude-sonnet-4".into()),
            });
        }
        if let Ok(key) = std::env::var("OPENAI_API_KEY") {
            return Some(Self::OpenAiCompat {
                api_key: key,
                base_url: std::env::var("OPENAI_BASE_URL")
                    .unwrap_or_else(|_| "https://api.openai.com/v1".into()),
                model: std::env::var("OPENAI_MODEL").unwrap_or_else(|_| "gpt-4.1".into()),
            });
        }
        if std::env::var("OLLAMA_HOST").is_ok()
            || std::path::Path::new("/usr/local/bin/ollama").exists()
        {
            return Some(Self::Ollama {
                base_url: std::env::var("OLLAMA_HOST").unwrap_or_else(|_| ollama_base()),
                model: std::env::var("OLLAMA_MODEL").unwrap_or_else(|_| "llama3.2".into()),
            });
        }
        None
    }

    pub fn uses_subscription_oauth(&self) -> bool {
        false
    }

    pub fn endpoint(&self) -> &str {
        match self {
            Self::AnthropicApi { base_url, .. } => base_url,
            Self::OpenAiCompat { base_url, .. } => base_url,
            Self::OpenRouter { base_url, .. } => base_url,
            Self::Ollama { base_url, .. } => base_url,
        }
    }

    fn api_key(&self) -> Option<&str> {
        match self {
            Self::AnthropicApi { api_key, .. }
            | Self::OpenAiCompat { api_key, .. }
            | Self::OpenRouter { api_key, .. } => Some(api_key.as_str()),
            Self::Ollama { .. } => None,
        }
    }

    fn model(&self) -> &str {
        match self {
            Self::AnthropicApi { model, .. } => model,
            Self::OpenAiCompat { model, .. } => model,
            Self::OpenRouter { model, .. } => model,
            Self::Ollama { model, .. } => model,
        }
    }
}

fn http_client() -> &'static reqwest::Client {
    static CLIENT: OnceLock<reqwest::Client> = OnceLock::new();
    CLIENT.get_or_init(|| {
        reqwest::Client::builder()
            .timeout(Duration::from_secs(90))
            .build()
            .expect("reqwest client")
    })
}

/// Call the provider. Never logs API keys or request bodies.
pub async fn complete(provider: &LlmProvider, messages: &[Value]) -> anyhow::Result<String> {
    if messages.is_empty() {
        anyhow::bail!("messages required");
    }
    if let Some(key) = provider.api_key() {
        if key.trim().is_empty() {
            anyhow::bail!("API key is empty");
        }
    }
    let plan = plan_chat_request(provider);
    tracing::info!(endpoint = %plan.url, oauth = false, "assistant chat");

    let mut req = http_client().post(&plan.url);
    match provider {
        LlmProvider::AnthropicApi { api_key, .. } => {
            req = req
                .header("x-api-key", api_key)
                .header("anthropic-version", "2023-06-01")
                .header("content-type", "application/json")
                .json(&anthropic_body(provider.model(), messages));
        }
        LlmProvider::OpenAiCompat { api_key, .. } | LlmProvider::OpenRouter { api_key, .. } => {
            req = req
                .bearer_auth(api_key)
                .header("content-type", "application/json")
                .json(&openai_body(provider.model(), messages));
        }
        LlmProvider::Ollama { .. } => {
            req = req.header("content-type", "application/json").json(&json!({
                "model": provider.model(),
                "messages": openai_messages(messages),
                "stream": false
            }));
        }
    }

    let resp = req.send().await.context("llm http")?;
    let status = resp.status();
    let v: Value = resp.json().await.context("llm json")?;
    if !status.is_success() {
        let snippet = v.to_string();
        let snippet = if snippet.len() > 240 {
            format!("{}…", &snippet[..240])
        } else {
            snippet
        };
        anyhow::bail!("llm HTTP {status}: {snippet}");
    }
    extract_text(provider, &v)
}

fn anthropic_body(model: &str, messages: &[Value]) -> Value {
    let mut system = String::new();
    let mut turns = Vec::new();
    for m in messages {
        let role = m.get("role").and_then(|r| r.as_str()).unwrap_or("user");
        let content = m
            .get("content")
            .and_then(|c| c.as_str())
            .unwrap_or("")
            .to_string();
        if role == "system" {
            if !system.is_empty() {
                system.push('\n');
            }
            system.push_str(&content);
        } else {
            let role = if role == "assistant" {
                "assistant"
            } else {
                "user"
            };
            turns.push(json!({"role": role, "content": content}));
        }
    }
    let mut body = json!({
        "model": model,
        "max_tokens": 1024,
        "messages": turns
    });
    if !system.is_empty() {
        body["system"] = json!(system);
    }
    body
}

fn openai_messages(messages: &[Value]) -> Vec<Value> {
    messages
        .iter()
        .map(|m| {
            json!({
                "role": m.get("role").and_then(|r| r.as_str()).unwrap_or("user"),
                "content": m.get("content").and_then(|c| c.as_str()).unwrap_or("")
            })
        })
        .collect()
}

fn openai_body(model: &str, messages: &[Value]) -> Value {
    json!({
        "model": model,
        "messages": openai_messages(messages)
    })
}

pub fn extract_text(provider: &LlmProvider, v: &Value) -> anyhow::Result<String> {
    let text = match provider {
        LlmProvider::AnthropicApi { .. } => v
            .get("content")
            .and_then(|c| c.as_array())
            .and_then(|c| c.first())
            .and_then(|c| c.get("text"))
            .and_then(|t| t.as_str()),
        LlmProvider::Ollama { .. } => v
            .get("message")
            .and_then(|m| m.get("content"))
            .and_then(|t| t.as_str()),
        LlmProvider::OpenAiCompat { .. } | LlmProvider::OpenRouter { .. } => v
            .get("choices")
            .and_then(|c| c.as_array())
            .and_then(|c| c.first())
            .and_then(|c| c.get("message"))
            .and_then(|m| m.get("content"))
            .and_then(|t| t.as_str()),
    };
    text.map(str::to_string)
        .ok_or_else(|| anyhow::anyhow!("unexpected llm response shape"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn providers_never_claim_oauth() {
        let p = LlmProvider::Ollama {
            base_url: ollama_base(),
            model: "llama3.2".into(),
        };
        assert!(!p.uses_subscription_oauth());
        assert!(p.endpoint().contains("11434"));
    }

    #[test]
    fn plan_chat_request_has_no_secrets() {
        let p = LlmProvider::AnthropicApi {
            api_key: "sk-secret".into(),
            base_url: anthropic_base(),
            model: anthropic_model(),
        };
        let plan = plan_chat_request(&p);
        let debug = format!("{plan:?}");
        assert!(!debug.contains("sk-secret"));
        assert_eq!(plan.auth, Some("x-api-key"));
        assert!(plan.url.ends_with("/v1/messages"));
    }

    #[test]
    fn extract_text_openai_shape() {
        let p = LlmProvider::OpenAiCompat {
            api_key: "k".into(),
            base_url: "https://api.openai.com/v1".into(),
            model: "gpt-4.1".into(),
        };
        let v = json!({"choices":[{"message":{"content":"hello"}}]});
        assert_eq!(extract_text(&p, &v).unwrap(), "hello");
    }

    #[test]
    fn extract_text_anthropic_shape() {
        let p = LlmProvider::AnthropicApi {
            api_key: "k".into(),
            base_url: anthropic_base(),
            model: anthropic_model(),
        };
        let v = json!({"content":[{"type":"text","text":"hi"}]});
        assert_eq!(extract_text(&p, &v).unwrap(), "hi");
    }
}
