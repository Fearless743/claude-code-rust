use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct McpClient {
    pub server_name: String,
    pub tools: Vec<crate::api::ToolDef>,
}
