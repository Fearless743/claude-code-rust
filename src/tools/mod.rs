pub mod ask;
pub mod bash;
pub mod dispatch;
pub mod file_edit;
pub mod file_read;
pub mod file_write;
pub mod glob;
pub mod grep;
pub mod permissions;
pub mod task;
pub mod web_fetch;
pub mod web_search;

use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolResult {
    pub content: String,
    pub is_error: bool,
}

#[derive(Debug, Clone)]
pub struct ToolContext {
    pub cwd: std::path::PathBuf,
    pub session_id: String,
}

#[async_trait]
pub trait Tool: Send + Sync {
    fn name(&self) -> &'static str;
    fn description(&self) -> &'static str;
    fn input_schema(&self) -> serde_json::Value;
    fn is_read_only(&self) -> bool;
    async fn call(&self, input: serde_json::Value, ctx: &ToolContext) -> eyre::Result<ToolResult>;
}

pub struct ToolRegistry {
    pub tools: HashMap<String, Box<dyn Tool>>,
}

impl ToolRegistry {
    pub fn new() -> Self {
        Self {
            tools: HashMap::new(),
        }
    }

    pub fn register(&mut self, tool: Box<dyn Tool>) {
        self.tools.insert(tool.name().to_string(), tool);
    }

    pub fn get(&self, name: &str) -> Option<&Box<dyn Tool>> {
        self.tools.get(name)
    }

    pub fn default_tools() -> Self {
        let mut registry = Self::new();
        registry.register(Box::new(bash::BashTool));
        registry.register(Box::new(file_read::FileReadTool));
        registry.register(Box::new(file_edit::FileEditTool));
        registry.register(Box::new(file_write::FileWriteTool));
        registry.register(Box::new(glob::GlobTool));
        registry.register(Box::new(grep::GrepTool));
        registry.register(Box::new(web_fetch::WebFetchTool));
        registry.register(Box::new(web_search::WebSearchTool));
        registry.register(Box::new(ask::AskUserQuestionTool));
        registry
    }
}
