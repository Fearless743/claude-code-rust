use crate::tools::{Tool, ToolContext, ToolResult};
use async_trait::async_trait;
use serde_json::json;

pub struct FileEditTool;

#[async_trait]
impl Tool for FileEditTool {
    fn name(&self) -> &'static str {
        "file_edit"
    }
    fn description(&self) -> &'static str {
        "Edit a file using search-and-replace. Provide the old text to find and \
         the new text to replace it with. The first exact match will be replaced."
    }
    fn input_schema(&self) -> serde_json::Value {
        json!({
            "type": "object",
            "properties": {
                "file_path": {"type": "string", "description": "Path to the file to edit"},
                "old_string": {"type": "string", "description": "The exact text to find and replace"},
                "new_string": {"type": "string", "description": "The text to replace it with"},
                "replace_all": {"type": "boolean", "description": "Replace all occurrences (default: false)"}
            },
            "required": ["file_path", "old_string", "new_string"]
        })
    }
    fn is_read_only(&self) -> bool {
        false
    }

    async fn call(&self, input: serde_json::Value, ctx: &ToolContext) -> eyre::Result<ToolResult> {
        let file_path = input["file_path"]
            .as_str()
            .ok_or_else(|| eyre::eyre!("Missing 'file_path'"))?;
        let old_string = input["old_string"]
            .as_str()
            .ok_or_else(|| eyre::eyre!("Missing 'old_string'"))?;
        let new_string = input["new_string"]
            .as_str()
            .ok_or_else(|| eyre::eyre!("Missing 'new_string'"))?;
        let replace_all = input["replace_all"].as_bool().unwrap_or(false);

        let path = if std::path::Path::new(file_path).is_absolute() {
            std::path::PathBuf::from(file_path)
        } else {
            ctx.cwd.join(file_path)
        };

        let original = tokio::fs::read_to_string(&path)
            .await
            .map_err(|e| eyre::eyre!("Failed to read {}: {e}", path.display()))?;

        if !replace_all && !original.contains(old_string) {
            return Ok(ToolResult {
                content: format!(
                    "Error: Could not find the exact string to replace in {}.\n\
                     The file contains {} lines. Verify the search string matches exactly.",
                    path.display(),
                    original.lines().count()
                ),
                is_error: true,
            });
        }

        let result = if replace_all {
            original.replace(old_string, new_string)
        } else {
            original.replacen(old_string, new_string, 1)
        };

        if result == original {
            return Ok(ToolResult {
                content: "No changes made — file content unchanged.".to_string(),
                is_error: false,
            });
        }

        let diff = similar::TextDiff::from_lines(&original, &result);
        let mut unified = diff.unified_diff();
        let header = format!("--- a/{}\n+++ b/{}\n", path.display(), path.display());
        let diff_text: String = unified.context_radius(3).to_string();

        tokio::fs::write(&path, &result)
            .await
            .map_err(|e| eyre::eyre!("Failed to write {}: {e}", path.display()))?;

        let line_count = result.lines().count();
        Ok(ToolResult {
            content: format!(
                "Successfully edited {} ({line_count} lines)\n\n{header}{diff_text}",
                path.display()
            ),
            is_error: false,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[tokio::test]
    async fn test_file_edit_single_occurrence() {
        let dir = TempDir::new().unwrap();
        let file_path = dir.path().join("test.txt");
        std::fs::write(&file_path, "hello world\nfoo bar\n").unwrap();

        let tool = FileEditTool;
        let ctx = ToolContext {
            cwd: dir.path().to_path_buf(),
            session_id: "test".into(),
        };
        let input = json!({
            "file_path": "test.txt",
            "old_string": "hello world",
            "new_string": "goodbye world"
        });
        let result = tool.call(input, &ctx).await.unwrap();
        assert!(!result.is_error);
        let content = std::fs::read_to_string(&file_path).unwrap();
        assert!(content.contains("goodbye world"));
    }

    #[tokio::test]
    async fn test_file_edit_not_found() {
        let dir = TempDir::new().unwrap();
        let file_path = dir.path().join("test.txt");
        std::fs::write(&file_path, "hello\n").unwrap();
        let tool = FileEditTool;
        let ctx = ToolContext {
            cwd: dir.path().to_path_buf(),
            session_id: "test".into(),
        };
        let input =
            json!({"file_path": "test.txt", "old_string": "nonexistent", "new_string": "x"});
        let result = tool.call(input, &ctx).await.unwrap();
        assert!(result.is_error);
    }
}
