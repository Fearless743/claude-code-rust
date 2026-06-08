pub mod compaction;
pub mod query;
pub mod session;

use eyre::Result;
use std::sync::Arc;

use crate::api::message::{ContentBlock, Message};
use crate::api::{self, Provider, RequestConfig};
use crate::config::Settings;
use crate::tools::ToolContext;
use crate::tools::dispatch::ToolDispatcher;

pub struct QueryEngine {
    provider: Arc<Box<dyn Provider>>,
    tools: ToolDispatcher,
    settings: Settings,
    system_prompt: String,
    cwd: std::path::PathBuf,
    max_turns: u32,
}

impl QueryEngine {
    pub async fn new(settings: Settings, provider: Option<Box<dyn Provider>>) -> Result<Self> {
        let provider = match provider {
            Some(p) => p,
            None => api::get_provider(&settings).await?,
        };

        let registry = crate::tools::ToolRegistry::default_tools();
        let tools = ToolDispatcher::new(registry);
        let cwd = std::env::current_dir().unwrap_or_else(|_| std::path::PathBuf::from("."));

        Ok(Self {
            provider: Arc::new(provider),
            tools,
            settings,
            system_prompt: String::new(),
            cwd,
            max_turns: 20,
        })
    }

    pub fn with_system_prompt(mut self, prompt: String) -> Self {
        self.system_prompt = prompt;
        self
    }

    #[allow(dead_code)]
    pub fn with_max_turns(mut self, turns: u32) -> Self {
        self.max_turns = turns;
        self
    }

    pub async fn run(self, prompt: Option<String>) -> Result<Vec<Message>> {
        let mut all_messages: Vec<Message> = Vec::new();
        let user_text = Self::read_user_prompt(prompt)?;

        let user_msg = Message::User {
            id: uuid::Uuid::new_v4(),
            content: vec![ContentBlock::Text { text: user_text }],
            timestamp: chrono::Utc::now(),
        };
        all_messages.push(user_msg.clone());
        let mut conversation: Vec<Message> = vec![user_msg];

        for _turn in 0..self.max_turns {
            let model = self.settings.resolve_model();
            let config = RequestConfig {
                model,
                max_tokens: 8192,
                temperature: 0.7,
                thinking_enabled: true,
            };

            let tools: Vec<api::ToolDef> = self
                .tools
                .registry()
                .tools
                .iter()
                .map(|(name, tool)| api::ToolDef {
                    name: name.clone(),
                    description: tool.description().to_string(),
                    input_schema: tool.input_schema(),
                })
                .collect();

            let mut stream = self
                .provider
                .stream_completion(conversation.clone(), &self.system_prompt, &tools, &config)
                .await?;

            use futures::StreamExt;
            let mut tool_calls: Vec<(String, String, serde_json::Value)> = Vec::new();

            while let Some(event) = stream.next().await {
                match event {
                    Ok(msg) => {
                        if let Message::Assistant { content, .. } = &msg {
                            for block in content {
                                if let ContentBlock::ToolUse { id, name, input } = block {
                                    tool_calls.push((id.clone(), name.clone(), input.clone()));
                                }
                            }
                        }
                        all_messages.push(msg.clone());
                        conversation.push(msg);
                    }
                    Err(e) => {
                        eprintln!("Stream error: {e}");
                        break;
                    }
                }
            }

            if !tool_calls.is_empty() {
                let ctx = ToolContext {
                    cwd: self.cwd.clone(),
                    session_id: "default".to_string(),
                };

                let parallel_calls: Vec<(String, serde_json::Value)> = tool_calls
                    .iter()
                    .map(|(_id, name, input)| (name.clone(), input.clone()))
                    .collect();

                let results = self.tools.execute_parallel(parallel_calls, &ctx).await;

                for ((tool_use_id, _name, _input), result) in tool_calls.iter().zip(results.iter())
                {
                    let content = match result {
                        Ok(r) => r.content.clone(),
                        Err(e) => format!("Error: {e}"),
                    };
                    let is_error = result.as_ref().map(|r| r.is_error).unwrap_or(true);

                    let tool_result = Message::User {
                        id: uuid::Uuid::new_v4(),
                        content: vec![ContentBlock::ToolResult {
                            tool_use_id: tool_use_id.clone(),
                            content,
                            is_error,
                        }],
                        timestamp: chrono::Utc::now(),
                    };

                    all_messages.push(tool_result.clone());
                    conversation.push(tool_result);
                }
                continue;
            }

            break;
        }

        Ok(all_messages)
    }

    fn read_user_prompt(prompt: Option<String>) -> Result<String> {
        if let Some(text) = prompt {
            Ok(text)
        } else {
            let mut input = String::new();
            std::io::Read::read_to_string(&mut std::io::stdin(), &mut input)?;
            let input = input.trim().to_string();
            if input.is_empty() {
                return Err(eyre::eyre!("No input provided"));
            }
            Ok(input)
        }
    }
}
