use crate::tools::{Tool, ToolContext, ToolResult};
use async_trait::async_trait;
use serde_json::json;

pub struct AskUserQuestionTool;

#[async_trait]
impl Tool for AskUserQuestionTool {
    fn name(&self) -> &'static str {
        "ask_user_question"
    }
    fn description(&self) -> &'static str {
        "Ask the user a multiple-choice or open-ended question. \
         Use when you need clarification before proceeding."
    }
    fn input_schema(&self) -> serde_json::Value {
        json!({
            "type": "object",
            "properties": {
                "question": {
                    "type": "string",
                    "description": "The question to ask the user"
                },
                "options": {
                    "type": "array",
                    "items": {
                        "type": "object",
                        "properties": {
                            "label": {"type": "string"},
                            "description": {"type": "string"}
                        }
                    },
                    "description": "Optional multiple-choice options"
                }
            },
            "required": ["question"]
        })
    }
    fn is_read_only(&self) -> bool {
        true
    }

    async fn call(&self, input: serde_json::Value, _ctx: &ToolContext) -> eyre::Result<ToolResult> {
        let question = input["question"]
            .as_str()
            .ok_or_else(|| eyre::eyre!("Missing 'question' parameter"))?;

        let mut result = format!("Question: {question}\n");
        if let Some(options) = input["options"].as_array() {
            for (i, opt) in options.iter().enumerate() {
                let label = opt["label"].as_str().unwrap_or("?");
                let desc = opt["description"].as_str().unwrap_or("");
                result.push_str(&format!("  {}. {label} - {desc}\n", i + 1));
            }
        }
        result.push_str("\n[Waiting for user response...]");

        Ok(ToolResult {
            content: result,
            is_error: false,
        })
    }
}
