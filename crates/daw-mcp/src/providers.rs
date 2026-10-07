//! Official API / OpenRouter / Ollama only. Never read ~/.claude or wrap `claude -p`.

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum LlmProvider {
    AnthropicApi {
        api_key: String,
        #[serde(default = "anthropic_base")]
        base_url: String,
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
fn openrouter_base() -> String {
    "https://openrouter.ai/api/v1".into()
}
fn ollama_base() -> String {
    "http://127.0.0.1:11434".into()
}

impl LlmProvider {
    pub fn from_env() -> Option<Self> {
        if let Ok(key) = std::env::var("ANTHROPIC_API_KEY") {
            return Some(Self::AnthropicApi {
                api_key: key,
                base_url: std::env::var("ANTHROPIC_BASE_URL").unwrap_or_else(|_| anthropic_base()),
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
}
