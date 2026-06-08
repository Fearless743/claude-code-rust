use async_trait::async_trait;
use serde_json::json;

use super::{Tool, ToolContext, ToolResult};

pub struct TaskTool;

#[async_trait]
impl Tool for TaskTool {
    fn name(&self) -> &'static str {
        "task"
    }
    fn description(&self) -> &'static str {
        "TODO: describe task"
    }
    fn input_schema(&self) -> serde_json::Value {
        json!({})
    }
    fn is_read_only(&self) -> bool {
        false
    }

    async fn call(
        &self,
        _input: serde_json::Value,
        _ctx: &ToolContext,
    ) -> eyre::Result<ToolResult> {
        Ok(ToolResult {
            content: format!("task: not yet implemented"),
            is_error: false,
        })
    }
}
