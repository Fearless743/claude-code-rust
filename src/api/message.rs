use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "role")]
pub enum Message {
    #[serde(rename = "user")]
    User {
        id: Uuid,
        content: Vec<ContentBlock>,
        timestamp: chrono::DateTime<chrono::Utc>,
    },
    #[serde(rename = "assistant")]
    Assistant {
        id: Uuid,
        content: Vec<ContentBlock>,
        model: String,
        stop_reason: Option<String>,
        usage: Option<Usage>,
        timestamp: chrono::DateTime<chrono::Utc>,
    },
    #[serde(rename = "system")]
    System {
        id: Uuid,
        content: String,
        timestamp: chrono::DateTime<chrono::Utc>,
    },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum ContentBlock {
    #[serde(rename = "text")]
    Text { text: String },
    #[serde(rename = "tool_use")]
    ToolUse {
        id: String,
        name: String,
        input: serde_json::Value,
    },
    #[serde(rename = "tool_result")]
    ToolResult {
        tool_use_id: String,
        content: String,
        is_error: bool,
    },
    #[serde(rename = "thinking")]
    Thinking { thinking: String, signature: String },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Usage {
    pub input_tokens: u64,
    pub output_tokens: u64,
    pub cache_creation_input_tokens: Option<u64>,
    pub cache_read_input_tokens: Option<u64>,
}
