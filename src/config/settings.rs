use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct Settings {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub api_key: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub base_url: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub provider: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub model: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub permission_mode: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub mcp_servers: Option<Vec<McpServerConfig>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct McpServerConfig {
    pub name: String,
    pub command: String,
    #[serde(default)]
    pub args: Vec<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub env: Option<HashMap<String, String>>,
}

impl Settings {
    pub fn resolve_api_key(&self) -> Option<String> {
        self.api_key
            .clone()
            .or_else(|| std::env::var("ANTHROPIC_API_KEY").ok())
    }

    pub fn resolve_base_url(&self) -> String {
        self.base_url
            .clone()
            .or_else(|| std::env::var("ANTHROPIC_BASE_URL").ok())
            .unwrap_or_else(|| "https://api.anthropic.com".to_string())
    }

    pub fn resolve_provider(&self) -> String {
        self.provider
            .clone()
            .or_else(|| {
                if std::env::var("CLAUDE_CODE_USE_OPENAI").ok().as_deref() == Some("1") {
                    Some("openai".to_string())
                } else if std::env::var("CLAUDE_CODE_USE_GEMINI").ok().as_deref() == Some("1") {
                    Some("gemini".to_string())
                } else if std::env::var("CLAUDE_CODE_USE_GROK").ok().as_deref() == Some("1") {
                    Some("grok".to_string())
                } else if std::env::var("CLAUDE_CODE_USE_BEDROCK").ok().as_deref() == Some("1") {
                    Some("bedrock".to_string())
                } else if std::env::var("CLAUDE_CODE_USE_VERTEX").ok().as_deref() == Some("1") {
                    Some("vertex".to_string())
                } else {
                    None
                }
            })
            .unwrap_or_else(|| "anthropic".to_string())
    }

    pub fn resolve_model(&self) -> String {
        self.model
            .clone()
            .or_else(|| std::env::var("ANTHROPIC_DEFAULT_SONNET_MODEL").ok())
            .or_else(|| std::env::var("ANTHROPIC_MODEL").ok())
            .unwrap_or_else(|| "claude-sonnet-4-20250514".to_string())
    }
}
