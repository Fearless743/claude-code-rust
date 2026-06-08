use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModelInfo {
    pub id: String,
    pub display_name: String,
    pub max_tokens: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ApiResponse {
    pub id: String,
    pub model: String,
    pub content: Vec<super::message::ContentBlock>,
    pub stop_reason: Option<String>,
    pub usage: super::message::Usage,
}
