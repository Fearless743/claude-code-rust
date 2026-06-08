pub mod anthropic;
pub mod bedrock;
pub mod foundry;
pub mod gemini;
pub mod grok;
pub mod message;
pub mod openai;
pub mod types;
pub mod vertex;

use async_trait::async_trait;
use futures::Stream;
use serde::{Deserialize, Serialize};
use std::pin::Pin;
use thiserror::Error;

use crate::config::Settings;

#[derive(Error, Debug)]
pub enum ApiError {
    #[error("HTTP request failed: {0}")]
    Http(#[from] reqwest::Error),

    #[error("HTTP {status}: {body}")]
    HttpError { status: u16, body: String },

    #[error("Auth error: {0}")]
    Auth(String),

    #[error("Rate limited")]
    RateLimited,

    #[error("Stream error: {0}")]
    Stream(String),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolDef {
    pub name: String,
    pub description: String,
    pub input_schema: serde_json::Value,
}

#[derive(Debug, Clone)]
pub struct RequestConfig {
    pub model: String,
    pub max_tokens: u32,
    pub temperature: f64,
    pub thinking_enabled: bool,
}

impl Default for RequestConfig {
    fn default() -> Self {
        Self {
            model: "claude-sonnet-4-20250514".to_string(),
            max_tokens: 8192,
            temperature: 0.7,
            thinking_enabled: true,
        }
    }
}

#[async_trait]
pub trait Provider: Send + Sync {
    async fn stream_completion(
        &self,
        messages: Vec<message::Message>,
        system_prompt: &str,
        tools: &[ToolDef],
        config: &RequestConfig,
    ) -> Result<Pin<Box<dyn Stream<Item = Result<message::Message, ApiError>> + Send>>, ApiError>;

    async fn supports_model(&self, model: &str) -> bool;
}

pub async fn get_provider(settings: &Settings) -> Result<Box<dyn Provider>, ApiError> {
    let provider = settings.resolve_provider();
    match provider.as_str() {
        "bedrock" => Ok(Box::new(bedrock::BedrockProvider::new(settings)?)),
        "vertex" => Ok(Box::new(vertex::VertexProvider::new(settings)?)),
        "openai" => Ok(Box::new(openai::OpenaiProvider::new(settings)?)),
        "gemini" => Ok(Box::new(gemini::GeminiProvider::new(settings)?)),
        "grok" => Ok(Box::new(grok::GrokProvider::new(settings)?)),
        _ => Ok(Box::new(anthropic::AnthropicProvider::new(settings)?)),
    }
}
