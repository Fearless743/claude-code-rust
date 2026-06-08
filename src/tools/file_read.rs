use crate::tools::{Tool, ToolContext, ToolResult};
use async_trait::async_trait;
use serde_json::json;

pub struct FileReadTool;

#[async_trait]
impl Tool for FileReadTool {
    fn name(&self) -> &'static str {
        "file_read"
    }
    fn description(&self) -> &'static str {
        "Read the contents of a file. Supports reading the entire file or specific line ranges."
    }
    fn input_schema(&self) -> serde_json::Value {
        json!({
            "type": "object",
            "properties": {
                "file_path": {
                    "type": "string",
                    "description": "Path to the file to read (absolute or relative to CWD)"
                },
                "offset": {
                    "type": "integer",
                    "description": "Line number to start reading from (0-indexed)"
                },
                "limit": {
                    "type": "integer",
                    "description": "Maximum number of lines to read"
                }
            },
            "required": ["file_path"]
        })
    }
    fn is_read_only(&self) -> bool {
        true
    }

    async fn call(&self, input: serde_json::Value, ctx: &ToolContext) -> eyre::Result<ToolResult> {
        let file_path = input["file_path"]
            .as_str()
            .ok_or_else(|| eyre::eyre!("Missing 'file_path' parameter"))?;

        let path = std::path::Path::new(file_path);
        let path = if path.is_absolute() {
            path.to_path_buf()
        } else {
            ctx.cwd.join(path)
        };

        let content = tokio::fs::read_to_string(&path)
            .await
            .map_err(|e| eyre::eyre!("Failed to read {}: {e}", path.display()))?;

        let lines: Vec<&str> = content.lines().collect();
        let total_lines = lines.len();
        let path_display = path.display();

        let offset = input["offset"].as_u64().unwrap_or(0) as usize;
        let limit = input["limit"].as_u64().map(|l| l as usize);

        let start = offset.min(total_lines);
        let end = limit
            .map(|l| (start + l).min(total_lines))
            .unwrap_or(total_lines);

        let selected: Vec<String> = lines[start..end]
            .iter()
            .enumerate()
            .map(|(i, line)| format!("{:>6} | {}", start + i + 1, line))
            .collect();

        let mut result = if offset > 0 || limit.is_some() {
            format!(
                "File: {path_display} (lines {}-{} of {total_lines})\n",
                start + 1,
                end
            )
        } else {
            format!("File: {path_display} ({total_lines} lines)\n")
        };
        result.push_str(&selected.join("\n"));

        Ok(ToolResult {
            content: result,
            is_error: false,
        })
    }
}
