use crate::tools::{Tool, ToolContext, ToolResult};
use async_trait::async_trait;
use serde_json::json;

pub struct WebSearchTool;

#[async_trait]
impl Tool for WebSearchTool {
    fn name(&self) -> &'static str {
        "web_search"
    }
    fn description(&self) -> &'static str {
        "Search the web. Returns relevant results for a search query. \
         Use this to find current information, documentation, or answers."
    }
    fn input_schema(&self) -> serde_json::Value {
        json!({
            "type": "object",
            "properties": {
                "query": {
                    "type": "string",
                    "description": "The search query"
                },
                "num_results": {
                    "type": "integer",
                    "description": "Number of results to return (default: 10)"
                }
            },
            "required": ["query"]
        })
    }
    fn is_read_only(&self) -> bool {
        true
    }

    async fn call(&self, input: serde_json::Value, _ctx: &ToolContext) -> eyre::Result<ToolResult> {
        let query = input["query"]
            .as_str()
            .ok_or_else(|| eyre::eyre!("Missing 'query' parameter"))?;

        Ok(ToolResult {
            content: format!(
                "Web search is not yet configured. Search query: '{query}'\n\
                 To enable web search, configure a search API (e.g. Tavily, Brave Search).\n\
                 Suggested: Add search_api_key to your config."
            ),
            is_error: false,
        })
    }
}
