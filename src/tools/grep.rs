use crate::tools::{Tool, ToolContext, ToolResult};
use async_trait::async_trait;
use regex::Regex;
use serde_json::json;

pub struct GrepTool;

#[async_trait]
impl Tool for GrepTool {
    fn name(&self) -> &'static str {
        "grep"
    }
    fn description(&self) -> &'static str {
        "Search for a regex pattern in files. Returns matching lines with file paths \
         and line numbers. Supports full regex syntax."
    }
    fn input_schema(&self) -> serde_json::Value {
        json!({
            "type": "object",
            "properties": {
                "pattern": {
                    "type": "string",
                    "description": "Regular expression pattern to search for"
                },
                "path": {
                    "type": "string",
                    "description": "Directory to search in (defaults to CWD)"
                },
                "include": {
                    "type": "string",
                    "description": "Glob pattern to filter files (e.g. '*.rs')"
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
        let re = Regex::new(pattern).map_err(|e| eyre::eyre!("Invalid regex pattern: {e}"))?;

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

        let include_pattern = input["include"].as_str();

        let mut results: Vec<String> = Vec::new();
        let mut total_matches = 0;
        let max_results = 10_000;

        for entry in walkdir::WalkDir::new(&search_path)
            .max_depth(20)
            .into_iter()
            .filter_entry(|e| {
                let name = e.file_name().to_string_lossy();
                !name.starts_with('.') && name != "target" && name != "node_modules"
            })
            .flatten()
        {
            if !entry.file_type().is_file() {
                continue;
            }

            if let Some(glob) = include_pattern {
                let rel = entry
                    .path()
                    .strip_prefix(&search_path)
                    .unwrap_or(entry.path());
                if !globset::Glob::new(glob)
                    .map(|g| g.compile_matcher().is_match(rel))
                    .unwrap_or(true)
                {
                    continue;
                }
            }

            if total_matches >= max_results {
                break;
            }

            let content = match tokio::fs::read_to_string(entry.path()).await {
                Ok(c) => c,
                Err(_) => continue,
            };

            let rel = entry
                .path()
                .strip_prefix(&search_path)
                .unwrap_or(entry.path());
            for (i, line) in content.lines().enumerate() {
                if re.is_match(line) {
                    results.push(format!("{}:{}: {}", rel.display(), i + 1, line));
                    total_matches += 1;
                    if total_matches >= max_results {
                        break;
                    }
                }
            }
        }

        let truncated = total_matches >= max_results;
        let mut output = format!("Found {} matches for '{}':\n", total_matches, pattern);
        output.push_str(&results.join("\n"));
        if truncated {
            output.push_str("\n\n... (results truncated at 10,000)");
        }

        Ok(ToolResult {
            content: output,
            is_error: false,
        })
    }
}
