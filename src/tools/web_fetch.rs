use crate::tools::{Tool, ToolContext, ToolResult};
use async_trait::async_trait;
use serde_json::json;

pub struct WebFetchTool;

#[async_trait]
impl Tool for WebFetchTool {
    fn name(&self) -> &'static str {
        "web_fetch"
    }
    fn description(&self) -> &'static str {
        "Fetch content from a URL. Returns the text content of a web page. \
         Use this to read documentation, API responses, or any web resource."
    }
    fn input_schema(&self) -> serde_json::Value {
        json!({
            "type": "object",
            "properties": {
                "url": {
                    "type": "string",
                    "description": "The URL to fetch content from"
                },
                "max_length": {
                    "type": "integer",
                    "description": "Maximum number of characters to return (default: 10000)"
                }
            },
            "required": ["url"]
        })
    }
    fn is_read_only(&self) -> bool {
        true
    }

    async fn call(&self, input: serde_json::Value, _ctx: &ToolContext) -> eyre::Result<ToolResult> {
        let url = input["url"]
            .as_str()
            .ok_or_else(|| eyre::eyre!("Missing 'url' parameter"))?;
        let max_length = input["max_length"].as_u64().unwrap_or(10000) as usize;

        let client = reqwest::Client::builder()
            .timeout(std::time::Duration::from_secs(30))
            .user_agent("claude-code-rust/2.1")
            .build()?;

        let response = client
            .get(url)
            .send()
            .await
            .map_err(|e| eyre::eyre!("Failed to fetch {url}: {e}"))?;

        let status = response.status();
        if !status.is_success() {
            return Ok(ToolResult {
                content: format!("HTTP {status} when fetching {url}"),
                is_error: true,
            });
        }

        let body = response
            .text()
            .await
            .map_err(|e| eyre::eyre!("Failed to read response body: {e}"))?;

        // Strip HTML tags for a basic text extraction
        let text = strip_html(&body);
        let truncated = text.len() > max_length;
        let result = if truncated {
            format!(
                "Content from {url} (truncated to {max_length} chars):\n\n{}...\n[truncated]",
                &text[..max_length.min(text.len())]
            )
        } else {
            format!("Content from {url}:\n\n{text}")
        };

        Ok(ToolResult {
            content: result,
            is_error: false,
        })
    }
}

fn strip_html(html: &str) -> String {
    let mut result = String::new();
    let mut in_tag = false;
    let mut in_script = false;
    let mut in_style = false;

    let lower = html.to_lowercase();
    let bytes = html.as_bytes();
    let mut i = 0;

    while i < bytes.len() {
        if !in_tag && bytes[i] == b'<' {
            // Check for script/style tags
            let remaining = &lower[i..];
            if remaining.starts_with("<script") {
                in_script = true;
                in_tag = true;
            } else if remaining.starts_with("<style") {
                in_style = true;
                in_tag = true;
            } else if remaining.starts_with("</script") {
                in_script = false;
                in_tag = true;
            } else if remaining.starts_with("</style") {
                in_style = false;
                in_tag = true;
            } else {
                in_tag = true;
            }
        } else if in_tag && bytes[i] == b'>' {
            in_tag = false;
            i += 1;
            continue;
        } else if !in_tag && !in_script && !in_style {
            result.push(html.as_bytes()[i] as char);
        }
        i += 1;
    }

    // Collapse whitespace
    let collapsed: Vec<&str> = result.split_whitespace().collect();
    collapsed.join(" ")
}
