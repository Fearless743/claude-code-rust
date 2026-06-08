use async_trait::async_trait;
use futures::stream::StreamExt;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::pin::Pin;
use tokio::sync::mpsc;

use super::message::{ContentBlock, Message, Usage};
use super::{ApiError, Provider, RequestConfig, ToolDef};
use crate::config::Settings;

const API_VERSION: &str = "2023-06-01";

#[derive(Debug, Serialize)]
struct AnthropicRequest {
    model: String,
    max_tokens: u32,
    messages: Vec<AnthropicMessage>,
    #[serde(skip_serializing_if = "Option::is_none")]
    system: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    tools: Option<Vec<AnthropicTool>>,
    stream: bool,
    temperature: f64,
}

#[derive(Debug, Serialize)]
struct AnthropicMessage {
    role: String,
    content: Value,
}

#[derive(Debug, Serialize)]
struct AnthropicTool {
    name: String,
    description: String,
    input_schema: Value,
}

#[derive(Debug, Deserialize)]
#[serde(tag = "type")]
enum AnthropicStreamEvent {
    #[serde(rename = "message_start")]
    MessageStart { message: AnthropicMessageStart },
    #[serde(rename = "content_block_start")]
    ContentBlockStart {
        index: usize,
        content_block: AnthropicContentBlockStart,
    },
    #[serde(rename = "content_block_delta")]
    ContentBlockDelta { index: usize, delta: AnthropicDelta },
    #[serde(rename = "content_block_stop")]
    ContentBlockStop { index: usize },
    #[serde(rename = "message_delta")]
    MessageDelta {
        delta: AnthropicMessageDelta,
        usage: AnthropicUsage,
    },
    #[serde(rename = "message_stop")]
    MessageStop,
    #[serde(rename = "ping")]
    Ping,
    #[serde(other)]
    Unknown,
}

#[derive(Debug, Deserialize)]
struct AnthropicMessageStart {
    model: String,
    usage: Option<AnthropicUsage>,
}

#[derive(Debug, Deserialize)]
#[serde(tag = "type")]
enum AnthropicContentBlockStart {
    #[serde(rename = "text")]
    Text { text: String },
    #[serde(rename = "tool_use")]
    ToolUse {
        id: String,
        name: String,
        input: Value,
    },
    #[serde(rename = "thinking")]
    Thinking { thinking: String },
}

#[derive(Debug, Deserialize)]
#[serde(tag = "type")]
enum AnthropicDelta {
    #[serde(rename = "text_delta")]
    TextDelta { text: String },
    #[serde(rename = "input_json_delta")]
    InputJsonDelta { partial_json: String },
    #[serde(rename = "thinking_delta")]
    ThinkingDelta { thinking: String },
    #[serde(rename = "signature_delta")]
    SignatureDelta { signature: String },
}

#[derive(Debug, Deserialize)]
struct AnthropicMessageDelta {
    stop_reason: Option<String>,
    stop_sequence: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
struct AnthropicUsage {
    input_tokens: Option<u64>,
    output_tokens: Option<u64>,
    cache_creation_input_tokens: Option<u64>,
    cache_read_input_tokens: Option<u64>,
}

pub struct AnthropicProvider {
    api_key: Option<String>,
    base_url: String,
    client: reqwest::Client,
}

impl AnthropicProvider {
    pub fn new(settings: &Settings) -> Result<Self, ApiError> {
        let api_key = settings.resolve_api_key();
        let base_url = settings.resolve_base_url();
        let client = reqwest::Client::builder()
            .timeout(std::time::Duration::from_secs(300))
            .build()
            .map_err(ApiError::Http)?;

        Ok(Self {
            api_key,
            base_url,
            client,
        })
    }

    fn convert_messages(&self, messages: &[Message]) -> Vec<AnthropicMessage> {
        messages
            .iter()
            .filter_map(|msg| match msg {
                Message::User { content, .. } => Some(AnthropicMessage {
                    role: "user".to_string(),
                    content: serde_json::to_value(content).unwrap_or(Value::String(String::new())),
                }),
                Message::Assistant { content, .. } => Some(AnthropicMessage {
                    role: "assistant".to_string(),
                    content: serde_json::to_value(content).unwrap_or(Value::String(String::new())),
                }),
                _ => None,
            })
            .collect()
    }

    fn convert_tools(&self, tools: &[ToolDef]) -> Vec<AnthropicTool> {
        tools
            .iter()
            .map(|t| AnthropicTool {
                name: t.name.clone(),
                description: t.description.clone(),
                input_schema: t.input_schema.clone(),
            })
            .collect()
    }

    fn build_request(
        &self,
        messages: &[Message],
        system_prompt: &str,
        tools: &[ToolDef],
        config: &RequestConfig,
    ) -> AnthropicRequest {
        let tools = if tools.is_empty() {
            None
        } else {
            Some(self.convert_tools(tools))
        };
        let system = if system_prompt.is_empty() {
            None
        } else {
            Some(system_prompt.to_string())
        };

        AnthropicRequest {
            model: config.model.clone(),
            max_tokens: config.max_tokens,
            messages: self.convert_messages(messages),
            system,
            tools,
            stream: true,
            temperature: config.temperature,
        }
    }
}

#[async_trait]
impl Provider for AnthropicProvider {
    async fn stream_completion(
        &self,
        messages: Vec<Message>,
        system_prompt: &str,
        tools: &[ToolDef],
        config: &RequestConfig,
    ) -> Result<Pin<Box<dyn futures::Stream<Item = Result<Message, ApiError>> + Send>>, ApiError>
    {
        let body = self.build_request(&messages, system_prompt, tools, config);

        let api_key = self
            .api_key
            .as_ref()
            .ok_or_else(|| ApiError::Auth("No API key configured".to_string()))?;

        let url = format!("{}/v1/messages", self.base_url.trim_end_matches('/'));

        let response = self
            .client
            .post(&url)
            .header("x-api-key", api_key.as_str())
            .header("anthropic-version", API_VERSION)
            .header("content-type", "application/json")
            .json(&body)
            .send()
            .await?;

        if !response.status().is_success() {
            let status = response.status().as_u16();
            let text = response.text().await.unwrap_or_default();
            return Err(ApiError::HttpError { status, body: text });
        }

        let mut byte_stream = response.bytes_stream();
        let (tx, rx) = mpsc::channel::<Result<Message, ApiError>>(64);

        tokio::spawn(async move {
            let mut buffer = String::new();
            let mut current_blocks: Vec<(String, String)> = Vec::new();
            let mut model = String::new();
            let mut stop_reason = None;
            let mut usage: Option<Usage> = None;

            while let Some(chunk_result) = byte_stream.next().await {
                let chunk = match chunk_result {
                    Ok(c) => c,
                    Err(e) => {
                        let _ = tx.send(Err(ApiError::Http(e))).await;
                        break;
                    }
                };
                let chunk_str = match std::str::from_utf8(&chunk) {
                    Ok(s) => s,
                    Err(_) => continue,
                };
                buffer.push_str(chunk_str);

                while let Some(pos) = buffer.find('\n') {
                    let line = buffer[..pos].trim().to_string();
                    buffer.drain(..=pos);

                    if line.is_empty() || !line.starts_with("data: ") {
                        continue;
                    }

                    let data = &line[6..];
                    if data == "[DONE]" {
                        continue;
                    }

                    let event: AnthropicStreamEvent =
                        serde_json::from_str(data).unwrap_or(AnthropicStreamEvent::Unknown);

                    match event {
                        AnthropicStreamEvent::MessageStart { message } => {
                            model = message.model;
                            usage = message.usage.map(|u| Usage {
                                input_tokens: u.input_tokens.unwrap_or(0),
                                output_tokens: 0,
                                cache_creation_input_tokens: u.cache_creation_input_tokens,
                                cache_read_input_tokens: u.cache_read_input_tokens,
                            });
                        }
                        AnthropicStreamEvent::ContentBlockStart {
                            index,
                            content_block,
                        } => {
                            while current_blocks.len() <= index {
                                current_blocks.push((String::new(), String::new()));
                            }
                            match content_block {
                                AnthropicContentBlockStart::Text { text } => {
                                    current_blocks[index].0 = text;
                                }
                                AnthropicContentBlockStart::ToolUse { id, name, .. } => {
                                    current_blocks[index].0 = id;
                                    current_blocks[index].1 = name;
                                }
                                AnthropicContentBlockStart::Thinking { thinking } => {
                                    current_blocks[index].0 = thinking;
                                }
                            }
                        }
                        AnthropicStreamEvent::ContentBlockDelta { index, delta } => {
                            while current_blocks.len() <= index {
                                current_blocks.push((String::new(), String::new()));
                            }
                            match delta {
                                AnthropicDelta::TextDelta { text } => {
                                    current_blocks[index].0.push_str(&text);
                                }
                                AnthropicDelta::InputJsonDelta { partial_json } => {
                                    current_blocks[index].1.push_str(&partial_json);
                                }
                                _ => {}
                            }
                        }
                        AnthropicStreamEvent::ContentBlockStop { .. } => {}
                        AnthropicStreamEvent::MessageDelta {
                            delta,
                            usage: msg_usage,
                        } => {
                            stop_reason = delta.stop_reason;
                            usage = Some(Usage {
                                input_tokens: msg_usage.input_tokens.unwrap_or(0),
                                output_tokens: msg_usage.output_tokens.unwrap_or(0),
                                cache_creation_input_tokens: msg_usage.cache_creation_input_tokens,
                                cache_read_input_tokens: msg_usage.cache_read_input_tokens,
                            });
                        }
                        AnthropicStreamEvent::MessageStop => {
                            let mut content = Vec::new();
                            for block in current_blocks.iter() {
                                if block.1.is_empty() {
                                    content.push(ContentBlock::Text {
                                        text: block.0.clone(),
                                    });
                                } else {
                                    content.push(ContentBlock::ToolUse {
                                        id: block.0.clone(),
                                        name: block.1.clone(),
                                        input: serde_json::from_str(&block.1)
                                            .unwrap_or(Value::Null),
                                    });
                                }
                            }

                            let _ = tx
                                .send(Ok(Message::Assistant {
                                    id: uuid::Uuid::new_v4(),
                                    content,
                                    model: model.clone(),
                                    stop_reason: stop_reason.clone(),
                                    usage: usage.clone(),
                                    timestamp: chrono::Utc::now(),
                                }))
                                .await;
                        }
                        AnthropicStreamEvent::Ping => {}
                        AnthropicStreamEvent::Unknown => {}
                    }
                }
            }
        });

        Ok(Box::pin(tokio_stream::wrappers::ReceiverStream::new(rx)))
    }

    async fn supports_model(&self, _model: &str) -> bool {
        true
    }
}
