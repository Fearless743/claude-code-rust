use crate::tools::{ToolContext, ToolRegistry, ToolResult};
use eyre::Result;
use serde_json::Value;
use std::sync::Arc;

pub struct ToolDispatcher {
    registry: Arc<ToolRegistry>,
}

impl ToolDispatcher {
    pub fn new(registry: ToolRegistry) -> Self {
        Self {
            registry: Arc::new(registry),
        }
    }

    pub fn registry(&self) -> &ToolRegistry {
        &self.registry
    }

    pub async fn execute(
        &self,
        tool_name: &str,
        input: Value,
        ctx: &ToolContext,
    ) -> Result<ToolResult> {
        let tool = self
            .registry
            .get(tool_name)
            .ok_or_else(|| eyre::eyre!("Unknown tool: {tool_name}"))?;
        tool.call(input, ctx).await
    }

    pub async fn execute_parallel(
        &self,
        calls: Vec<(String, Value)>,
        ctx: &ToolContext,
    ) -> Vec<Result<ToolResult>> {
        let mut handles = Vec::new();
        for (tool_name, input) in calls {
            let registry = self.registry.clone();
            let ctx = ctx.clone();
            handles.push(tokio::spawn(async move {
                let tool = registry.get(&tool_name);
                match tool {
                    Some(t) => t.call(input, &ctx).await,
                    None => Err(eyre::eyre!("Unknown tool: {tool_name}")),
                }
            }));
        }
        let mut results = Vec::new();
        for handle in handles {
            results.push(
                handle
                    .await
                    .unwrap_or_else(|e| Err(eyre::eyre!("Panic: {e}"))),
            );
        }
        results
    }
}
