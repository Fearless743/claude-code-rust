use crate::tools::{Tool, ToolContext, ToolResult};
use async_trait::async_trait;
use serde_json::json;

pub struct BashTool;

#[async_trait]
impl Tool for BashTool {
    fn name(&self) -> &'static str {
        "bash"
    }
    fn description(&self) -> &'static str {
        "Execute a bash shell command. Use this for system operations, file manipulation, \
         building projects, running tests, git operations, and any terminal commands."
    }
    fn input_schema(&self) -> serde_json::Value {
        json!({
            "type": "object",
            "properties": {
                "command": {
                    "type": "string",
                    "description": "The bash command to execute"
                },
                "description": {
                    "type": "string",
                    "description": "Brief description of what this command does (for permission dialogs)"
                }
            },
            "required": ["command"]
        })
    }
    fn is_read_only(&self) -> bool {
        false
    }

    async fn call(&self, input: serde_json::Value, ctx: &ToolContext) -> eyre::Result<ToolResult> {
        let command = input["command"]
            .as_str()
            .ok_or_else(|| eyre::eyre!("Missing 'command' parameter"))?;

        let timeout_secs = input["timeout"].as_u64().unwrap_or(120);

        let result = tokio::time::timeout(
            std::time::Duration::from_secs(timeout_secs),
            tokio::process::Command::new("bash")
                .arg("-c")
                .arg(command)
                .current_dir(&ctx.cwd)
                .output(),
        )
        .await
        .map_err(|_| eyre::eyre!("Command timed out after {timeout_secs}s"))??;

        let stdout = String::from_utf8_lossy(&result.stdout).to_string();
        let stderr = String::from_utf8_lossy(&result.stderr).to_string();
        let exit_code = result.status.code().unwrap_or(-1);

        let mut content = String::new();
        if !stdout.is_empty() {
            content.push_str(&stdout);
        }
        if !stderr.is_empty() {
            if !content.is_empty() {
                content.push('\n');
            }
            content.push_str("[stderr]\n");
            content.push_str(&stderr);
        }
        if content.is_empty() {
            content = format!("Command exited with code {exit_code}");
        }

        Ok(ToolResult {
            content,
            is_error: exit_code != 0,
        })
    }
}
