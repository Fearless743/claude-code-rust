use async_trait::async_trait;
use futures::StreamExt;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::pin::Pin;
use tokio::sync::mpsc;

use super::message::{ContentBlock, Message, Usage};
use super::{ApiError, Provider, RequestConfig, ToolDef};
use crate::config::Settings;

#[derive(Debug, Serialize)]
struct OpenAiRequest {
    model: String,
    messages: Vec<OpenAiMessage>,
    #[serde(skip_serializing_if = "Option::is_none")]
    tools: Option<Vec<OpenAiTool>>,
    stream: bool,
    max_tokens: u32,
    temperature: f64,
}

#[derive(Debug, Serialize)]
struct OpenAiMessage {
    role: String,
    content: Value,
    #[serde(skip_serializing_if = "Option::is_none")]
    tool_calls: Option<Vec<OpenAiToolCall>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    tool_call_id: Option<String>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
struct OpenAiToolCall {
    id: String,
    #[serde(rename = "type")]
    call_type: String,
    function: OpenAiFunctionCall,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
struct OpenAiFunctionCall {
    name: String,
    arguments: String,
}

#[derive(Debug, Serialize)]
struct OpenAiTool {
    #[serde(rename = "type")]
    tool_type: String,
    function: OpenAiToolFunction,
}

#[derive(Debug, Serialize)]
struct OpenAiToolFunction {
    name: String,
    description: String,
    parameters: Value,
}

#[derive(Debug, Deserialize)]
struct OpenAiStreamChunk {
    choices: Vec<OpenAiChoice>,
}

#[derive(Debug, Deserialize)]
struct OpenAiChoice {
    delta: OpenAiDelta,
    #[serde(default)]
    finish_reason: Option<String>,
}

#[derive(Debug, Deserialize)]
struct OpenAiDelta {
    #[serde(default)]
    content: Option<String>,
    #[serde(default)]
    tool_calls: Option<Vec<OpenAiToolCall>>,
}

pub struct OpenaiProvider {
    api_key: Option<String>,
    base_url: String,
    client: reqwest::Client,
}

impl OpenaiProvider {
    pub fn new(settings: &Settings) -> Result<Self, ApiError> {
        let api_key = settings
            .api_key
            .clone()
            .or_else(|| std::env::var("OPENAI_API_KEY").ok());
        let base_url = std::env::var("OPENAI_BASE_URL")
            .unwrap_or_else(|_| "https://api.openai.com".to_string());
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

    fn convert_messages(&self, messages: &[Message]) -> Vec<OpenAiMessage> {
        messages
            .iter()
            .map(|msg| match msg {
                Message::User { content, .. } => OpenAiMessage {
                    role: "user".into(),
                    content: serde_json::to_value(content).unwrap_or_default(),
                    tool_calls: None,
                    tool_call_id: None,
                },
                Message::Assistant { content, .. } => {
                    let mut text = String::new();
                    let mut tool_calls = Vec::new();
                    for block in content {
                        match block {
                            ContentBlock::Text { text: t } => text.push_str(t),
                            ContentBlock::ToolUse { id, name, input } => {
                                tool_calls.push(OpenAiToolCall {
                                    id: id.clone(),
                                    call_type: "function".into(),
                                    function: OpenAiFunctionCall {
                                        name: name.clone(),
                                        arguments: serde_json::to_string(input).unwrap_or_default(),
                                    },
                                });
                            }
                            _ => {}
                        }
                    }
                    OpenAiMessage {
                        role: "assistant".into(),
                        content: Value::String(text),
                        tool_calls: if tool_calls.is_empty() {
                            None
                        } else {
                            Some(tool_calls)
                        },
                        tool_call_id: None,
                    }
                }
                Message::System { content, .. } => OpenAiMessage {
                    role: "system".into(),
                    content: Value::String(content.clone()),
                    tool_calls: None,
                    tool_call_id: None,
                },
            })
            .collect()
    }

    fn convert_tools(&self, tools: &[ToolDef]) -> Vec<OpenAiTool> {
        tools
            .iter()
            .map(|t| OpenAiTool {
                tool_type: "function".into(),
                function: OpenAiToolFunction {
                    name: t.name.clone(),
                    description: t.description.clone(),
                    parameters: t.input_schema.clone(),
                },
            })
            .collect()
    }
}

#[async_trait]
impl Provider for OpenaiProvider {
    async fn stream_completion(
        &self,
        messages: Vec<Message>,
        system_prompt: &str,
        tools: &[ToolDef],
        config: &RequestConfig,
    ) -> Result<Pin<Box<dyn futures::Stream<Item = Result<Message, ApiError>> + Send>>, ApiError>
    {
        let api_key = self
            .api_key
            .as_ref()
            .ok_or_else(|| ApiError::Auth("No OpenAI API key".into()))?;

        // Insert system message if present
        let mut all_msgs = messages.clone();
        if !system_prompt.is_empty() {
            all_msgs.insert(
                0,
                Message::System {
                    id: uuid::Uuid::new_v4(),
                    content: system_prompt.to_string(),
                    timestamp: chrono::Utc::now(),
                },
            );
        }

        let body = OpenAiRequest {
            model: config.model.clone(),
            messages: self.convert_messages(&all_msgs),
            tools: if tools.is_empty() {
                None
            } else {
                Some(self.convert_tools(tools))
            },
            stream: true,
            max_tokens: config.max_tokens,
            temperature: config.temperature,
        };

        let url = format!(
            "{}/v1/chat/completions",
            self.base_url.trim_end_matches('/')
        );
        let response = self
            .client
            .post(&url)
            .bearer_auth(api_key)
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
            let mut text_accum = String::new();
            let mut tool_calls_acc: Vec<OpenAiToolCall> = Vec::new();
            let mut finish_reason: Option<String> = None;

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
                        let mut content = Vec::new();
                        if !text_accum.is_empty() {
                            content.push(ContentBlock::Text {
                                text: std::mem::take(&mut text_accum),
                            });
                        }
                        for tc in &tool_calls_acc {
                            content.push(ContentBlock::ToolUse {
                                id: tc.id.clone(),
                                name: tc.function.name.clone(),
                                input: serde_json::from_str(&tc.function.arguments)
                                    .unwrap_or(Value::Null),
                            });
                        }
                        let _ = tx
                            .send(Ok(Message::Assistant {
                                id: uuid::Uuid::new_v4(),
                                content,
                                model: String::new(),
                                stop_reason: finish_reason.take(),
                                usage: None,
                                timestamp: chrono::Utc::now(),
                            }))
                            .await;
                        continue;
                    }
                    if let Ok(chunk) = serde_json::from_str::<OpenAiStreamChunk>(data) {
                        for choice in &chunk.choices {
                            if let Some(t) = &choice.delta.content {
                                text_accum.push_str(t);
                            }
                            if let Some(tcs) = &choice.delta.tool_calls {
                                for tc in tcs {
                                    if let Some(existing) =
                                        tool_calls_acc.iter_mut().find(|x| x.id == tc.id)
                                    {
                                        existing
                                            .function
                                            .arguments
                                            .push_str(&tc.function.arguments);
                                    } else {
                                        tool_calls_acc.push(tc.clone());
                                    }
                                }
                            }
                            if let Some(fr) = &choice.finish_reason {
                                finish_reason = Some(fr.clone());
                            }
                        }
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
