use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;

use crate::api::ToolDef;
use crate::api::message::Message;

#[derive(Debug, Clone)]
pub struct AppState {
    pub session_id: String,
    pub messages: Vec<Message>,
    pub tools: Vec<ToolDef>,
    pub permission_mode: PermissionMode,
    pub mcp_clients: HashMap<String, McpClientState>,
    pub model: String,
    pub thinking_enabled: bool,
    pub total_cost_usd: f64,
    pub total_tokens: u64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PermissionMode {
    Default,
    AcceptEdits,
    BypassPermissions,
    Plan,
    Auto,
}

#[derive(Debug, Clone)]
pub struct McpClientState {
    pub name: String,
    pub connected: bool,
    pub tool_count: usize,
}

impl AppState {
    pub fn new(session_id: String) -> Self {
        Self {
            session_id,
            messages: Vec::new(),
            tools: Vec::new(),
            permission_mode: PermissionMode::Default,
            mcp_clients: HashMap::new(),
            model: "claude-sonnet-4-20250514".to_string(),
            thinking_enabled: true,
            total_cost_usd: 0.0,
            total_tokens: 0,
        }
    }
}

pub type SharedState = Arc<RwLock<AppState>>;

pub fn new_shared_state(session_id: String) -> SharedState {
    Arc::new(RwLock::new(AppState::new(session_id)))
}
