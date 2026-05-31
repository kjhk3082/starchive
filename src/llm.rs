//! Optional LLM integration (bring-your-own-key). Supports OpenRouter (default),
//! Anthropic Claude, and OpenAI — auto-detected from whichever API key is set in
//! the environment. The key is read from env only; it is never written to source
//! or committed. If no key is configured, [`Llm::from_env`] returns `None` and
//! all LLM features are simply hidden.

use serde_json::json;

use crate::error::{AppError, Result};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Provider {
    OpenRouter,
    Anthropic,
    OpenAI,
}

impl Provider {
    fn default_model(self) -> &'static str {
        match self {
            Provider::OpenRouter => "qwen/qwen3.5-plus-20260420",
            Provider::Anthropic => "claude-haiku-4-5-20251001",
            Provider::OpenAI => "gpt-4o-mini",
        }
    }
    pub fn label(self) -> &'static str {
        match self {
            Provider::OpenRouter => "OpenRouter",
            Provider::Anthropic => "Claude",
            Provider::OpenAI => "OpenAI",
        }
    }
}

#[derive(Clone)]
pub struct Llm {
    http: reqwest::Client,
    provider: Provider,
    api_key: String,
    model: String,
}

impl Llm {
    /// Build from the environment, or `None` if no provider key is present.
    pub fn from_env() -> Option<Self> {
        let explicit = std::env::var("STARCHIVE_LLM_PROVIDER").ok();
        let (provider, api_key) = match explicit.as_deref() {
            Some("openrouter") => (Provider::OpenRouter, non_empty("OPENROUTER_API_KEY")?),
            Some("anthropic") => (Provider::Anthropic, non_empty("ANTHROPIC_API_KEY")?),
            Some("openai") => (Provider::OpenAI, non_empty("OPENAI_API_KEY")?),
            _ => detect()?,
        };
        let model = std::env::var("STARCHIVE_LLM_MODEL")
            .ok()
            .filter(|m| !m.trim().is_empty())
            .unwrap_or_else(|| provider.default_model().to_string());

        Some(Self {
            http: reqwest::Client::new(),
            provider,
            api_key,
            model,
        })
    }

    pub fn provider(&self) -> Provider {
        self.provider
    }
    pub fn model(&self) -> &str {
        &self.model
    }

    /// Single-shot completion. `system` sets behavior; `user` is the request.
    pub async fn complete(&self, system: &str, user: &str, max_tokens: u32) -> Result<String> {
        match self.provider {
            Provider::Anthropic => self.anthropic(system, user, max_tokens).await,
            Provider::OpenRouter | Provider::OpenAI => {
                self.openai_compatible(system, user, max_tokens).await
            }
        }
    }

    async fn openai_compatible(&self, system: &str, user: &str, max_tokens: u32) -> Result<String> {
        let base = match self.provider {
            Provider::OpenRouter => "https://openrouter.ai/api/v1",
            Provider::OpenAI => "https://api.openai.com/v1",
            Provider::Anthropic => unreachable!(),
        };
        let body = json!({
            "model": self.model,
            "max_tokens": max_tokens,
            "temperature": 0.3,
            "messages": [
                {"role": "system", "content": system},
                {"role": "user", "content": user},
            ],
        });
        let resp = self
            .http
            .post(format!("{base}/chat/completions"))
            .bearer_auth(&self.api_key)
            .header("HTTP-Referer", "https://github.com/kjhk3082/starchive")
            .header("X-Title", "starchive")
            .json(&body)
            .send()
            .await?;
        let v = checked_json(resp).await?;
        v["choices"][0]["message"]["content"]
            .as_str()
            .map(|s| s.trim().to_string())
            .ok_or_else(|| AppError::msg("LLM returned an unexpected response shape"))
    }

    async fn anthropic(&self, system: &str, user: &str, max_tokens: u32) -> Result<String> {
        let body = json!({
            "model": self.model,
            "max_tokens": max_tokens,
            "system": system,
            "messages": [{"role": "user", "content": user}],
        });
        let resp = self
            .http
            .post("https://api.anthropic.com/v1/messages")
            .header("x-api-key", &self.api_key)
            .header("anthropic-version", "2023-06-01")
            .json(&body)
            .send()
            .await?;
        let v = checked_json(resp).await?;
        v["content"][0]["text"]
            .as_str()
            .map(|s| s.trim().to_string())
            .ok_or_else(|| AppError::msg("LLM returned an unexpected response shape"))
    }
}

fn non_empty(var: &str) -> Option<String> {
    std::env::var(var).ok().filter(|v| !v.trim().is_empty())
}

fn detect() -> Option<(Provider, String)> {
    if let Some(k) = non_empty("OPENROUTER_API_KEY") {
        Some((Provider::OpenRouter, k))
    } else if let Some(k) = non_empty("ANTHROPIC_API_KEY") {
        Some((Provider::Anthropic, k))
    } else {
        non_empty("OPENAI_API_KEY").map(|k| (Provider::OpenAI, k))
    }
}

async fn checked_json(resp: reqwest::Response) -> Result<serde_json::Value> {
    let status = resp.status();
    let text = resp.text().await?;
    if !status.is_success() {
        let snippet: String = text.chars().take(300).collect();
        return Err(AppError::msg(format!("LLM API {status}: {snippet}")));
    }
    serde_json::from_str(&text).map_err(AppError::from)
}
