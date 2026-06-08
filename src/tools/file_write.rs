use crate::tools::{Tool, ToolContext, ToolResult};
use async_trait::async_trait;
use serde_json::json;

pub struct FileWriteTool;

#[async_trait]
impl Tool for FileWriteTool {
    fn name(&self) -> &'static str {
        "file_write"
    }
    fn description(&self) -> &'static str {
        "Write content to a file, creating it if it doesn't exist. \
         Overwrites the file completely with the provided content."
    }
    fn input_schema(&self) -> serde_json::Value {
        json!({
            "type": "object",
            "properties": {
                "file_path": {
                    "type": "string",
                    "description": "Path to the file to write (absolute or relative to CWD)"
                },
                "content": {
                    "type": "string",
                    "description": "The content to write to the file"
                }
            },
            "required": ["file_path", "content"]
        })
    }
    fn is_read_only(&self) -> bool {
        false
    }

    async fn call(&self, input: serde_json::Value, ctx: &ToolContext) -> eyre::Result<ToolResult> {
        let file_path = input["file_path"]
            .as_str()
            .ok_or_else(|| eyre::eyre!("Missing 'file_path' parameter"))?;
        let content = input["content"]
            .as_str()
            .ok_or_else(|| eyre::eyre!("Missing 'content' parameter"))?;

        let path = std::path::Path::new(file_path);
        let path = if path.is_absolute() {
            path.to_path_buf()
        } else {
            ctx.cwd.join(path)
        };

        // Ensure parent directory exists
        if let Some(parent) = path.parent() {
            tokio::fs::create_dir_all(parent).await?;
        }

        tokio::fs::write(&path, content)
            .await
            .map_err(|e| eyre::eyre!("Failed to write {}: {e}", path.display()))?;

        let line_count = content.lines().count();
        let size = content.len();

        Ok(ToolResult {
            content: format!(
                "Successfully wrote {} ({} lines, {} bytes)",
                path.display(),
                line_count,
                size
            ),
            is_error: false,
        })
    }
}
