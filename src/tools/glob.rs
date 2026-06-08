use crate::tools::{Tool, ToolContext, ToolResult};
use async_trait::async_trait;
use globset::{Glob, GlobSetBuilder};
use serde_json::json;

pub struct GlobTool;

#[async_trait]
impl Tool for GlobTool {
    fn name(&self) -> &'static str {
        "glob"
    }
    fn description(&self) -> &'static str {
        "Find files matching a glob pattern. Uses standard glob syntax with \
         *, **, ?, [abc] support. Returns matching file paths."
    }
    fn input_schema(&self) -> serde_json::Value {
        json!({
            "type": "object",
            "properties": {
                "pattern": {
                    "type": "string",
                    "description": "Glob pattern to match (e.g. '**/*.rs', 'src/**/*.{js,ts}')"
                },
                "path": {
                    "type": "string",
                    "description": "Directory to search in (defaults to CWD)"
                }
            },
            "required": ["pattern"]
        })
    }
    fn is_read_only(&self) -> bool {
        true
    }

    async fn call(&self, input: serde_json::Value, ctx: &ToolContext) -> eyre::Result<ToolResult> {
        let pattern = input["pattern"]
            .as_str()
            .ok_or_else(|| eyre::eyre!("Missing 'pattern' parameter"))?;

        let search_path = if let Some(p) = input["path"].as_str() {
            let p = std::path::Path::new(p);
            if p.is_absolute() {
                p.to_path_buf()
            } else {
                ctx.cwd.join(p)
            }
        } else {
            ctx.cwd.clone()
        };

        let mut builder = GlobSetBuilder::new();
        let glob = Glob::new(pattern).map_err(|e| eyre::eyre!("Invalid glob pattern: {e}"))?;
        builder.add(glob);
        let set = builder
            .build()
            .map_err(|e| eyre::eyre!("Failed to build glob matcher: {e}"))?;

        let mut matches: Vec<String> = Vec::new();
        for entry in walkdir::WalkDir::new(&search_path)
            .max_depth(20)
            .into_iter()
            .filter_entry(|e| {
                let name = e.file_name().to_string_lossy();
                !name.starts_with('.') || name == "."
            })
            .flatten()
        {
            let rel = entry
                .path()
                .strip_prefix(&search_path)
                .unwrap_or(entry.path());
            if set.is_match(rel) {
                let display = rel.display().to_string();
                if !display.is_empty() {
                    matches.push(display);
                }
            }
        }

        matches.sort();
        let truncated = matches.len() > 500;
        if truncated {
            matches.truncate(500);
        }

        let mut result = format!("Found {} matches for '{}':\n", matches.len(), pattern);
        result.push_str(&matches.join("\n"));
        if truncated {
            result.push_str("\n\n... (results truncated at 500)");
        }

        Ok(ToolResult {
            content: result,
            is_error: false,
        })
    }
}
