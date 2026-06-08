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
struct GeminiRequest {
    contents: Vec<GeminiContent>,
    #[serde(skip_serializing_if = "Option::is_none")]
    system_instruction: Option<GeminiSystemInstruction>,
    #[serde(skip_serializing_if = "Option::is_none")]
    tools: Option<Vec<GeminiTool>>,
    #[serde(rename = "generationConfig")]
    generation_config: GeminiGenerationConfig,
}

#[derive(Debug, Serialize)]
struct GeminiContent {
    role: String,
    parts: Vec<GeminiPart>,
}

#[derive(Debug, Serialize)]
struct GeminiSystemInstruction {
    parts: Vec<GeminiTextPart>,
}

#[derive(Debug, Serialize)]
#[serde(untagged)]
enum GeminiPart {
    Text {
        text: String,
    },
    FunctionCall {
        function_call: GeminiFunctionCall,
    },
    FunctionResponse {
        function_response: GeminiFunctionResponse,
    },
}

#[derive(Debug, Serialize)]
struct GeminiTextPart {
    text: String,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
struct GeminiFunctionCall {
    name: String,
    args: Value,
}

#[derive(Debug, Serialize)]
struct GeminiFunctionResponse {
    name: String,
    response: Value,
}

#[derive(Debug, Serialize)]
struct GeminiTool {
    #[serde(rename = "functionDeclarations")]
    function_declarations: Vec<GeminiFunctionDeclaration>,
}

#[derive(Debug, Serialize)]
struct GeminiFunctionDeclaration {
    name: String,
    description: String,
    parameters: Value,
}

#[derive(Debug, Serialize)]
struct GeminiGenerationConfig {
    #[serde(rename = "maxOutputTokens")]
    max_output_tokens: u32,
    temperature: f64,
}

#[derive(Debug, Deserialize)]
struct GeminiStreamChunk {
    candidates: Option<Vec<GeminiCandidate>>,
}

#[derive(Debug, Deserialize)]
struct GeminiCandidate {
    content: Option<GeminiContentResponse>,
    #[serde(rename = "finishReason")]
    finish_reason: Option<String>,
}

#[derive(Debug, Deserialize)]
struct GeminiContentResponse {
    parts: Option<Vec<GeminiPartResponse>>,
    role: Option<String>,
}

#[derive(Debug, Deserialize)]
#[serde(untagged)]
enum GeminiPartResponse {
    Text { text: String },
    FunctionCall { function_call: GeminiFunctionCall },
}

pub struct GeminiProvider {
    api_key: Option<String>,
    base_url: String,
    client: reqwest::Client,
}

impl GeminiProvider {
    pub fn new(settings: &Settings) -> Result<Self, ApiError> {
        let api_key = settings
            .api_key
            .clone()
            .or_else(|| std::env::var("GEMINI_API_KEY").ok());
        let base_url = "https://generativelanguage.googleapis.com".to_string();
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

    fn convert_messages(&self, messages: &[Message]) -> Vec<GeminiContent> {
        messages
            .iter()
            .filter_map(|msg| match msg {
                Message::User { content, .. } => {
                    let text: String = content
                        .iter()
                        .filter_map(|b| match b {
                            ContentBlock::Text { text } => Some(text.as_str()),
                            _ => None,
                        })
                        .collect::<Vec<_>>()
                        .join("\n");
                    Some(GeminiContent {
                        role: "user".into(),
                        parts: vec![GeminiPart::Text { text }],
                    })
                }
                Message::Assistant { content, .. } => {
                    let mut parts = Vec::new();
                    for block in content {
                        match block {
                            ContentBlock::Text { text } => {
                                parts.push(GeminiPart::Text { text: text.clone() });
                            }
                            ContentBlock::ToolUse { name, input, .. } => {
                                parts.push(GeminiPart::FunctionCall {
                                    function_call: GeminiFunctionCall {
                                        name: name.clone(),
                                        args: input.clone(),
                                    },
                                });
                            }
                            _ => {}
                        }
                    }
                    Some(GeminiContent {
                        role: "model".into(),
                        parts,
                    })
                }
                _ => None,
            })
            .collect()
    }

    fn convert_tools(&self, tools: &[ToolDef]) -> Vec<GeminiTool> {
        let decls: Vec<GeminiFunctionDeclaration> = tools
            .iter()
            .map(|t| GeminiFunctionDeclaration {
                name: t.name.clone(),
                description: t.description.clone(),
                parameters: t.input_schema.clone(),
            })
            .collect();
        vec![GeminiTool {
            function_declarations: decls,
        }]
    }
}

#[async_trait]
impl Provider for GeminiProvider {
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
            .ok_or_else(|| ApiError::Auth("No Gemini API key".into()))?;

        let system_instruction = if system_prompt.is_empty() {
            None
        } else {
            Some(GeminiSystemInstruction {
                parts: vec![GeminiTextPart {
                    text: system_prompt.to_string(),
                }],
            })
        };

        let body = GeminiRequest {
            contents: self.convert_messages(&messages),
            system_instruction,
            tools: if tools.is_empty() {
                None
            } else {
                Some(self.convert_tools(tools))
            },
            generation_config: GeminiGenerationConfig {
                max_output_tokens: config.max_tokens,
                temperature: config.temperature,
            },
        };

        let model = config.model.clone();
        let url = format!(
            "{}/v1beta/models/{}:streamGenerateContent?alt=sse&key={}",
            self.base_url.trim_end_matches('/'),
            model,
            api_key
        );

        let response = self.client.post(&url).json(&body).send().await?;

        if !response.status().is_success() {
            let status = response.status().as_u16();
            let text = response.text().await.unwrap_or_default();
            return Err(ApiError::HttpError { status, body: text });
        }

        let mut byte_stream = response.bytes_stream();
        let (tx, rx) = mpsc::channel::<Result<Message, ApiError>>(64);

        tokio::spawn(async move {
            let mut buffer = String::new();
            let mut text_parts: Vec<String> = Vec::new();

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

                    if let Ok(chunk) = serde_json::from_str::<GeminiStreamChunk>(data) {
                        if let Some(candidates) = &chunk.candidates {
                            for candidate in candidates {
                                if let Some(content) = &candidate.content {
                                    if let Some(parts) = &content.parts {
                                        for part in parts {
                                            match part {
                                                GeminiPartResponse::Text { text } => {
                                                    text_parts.push(text.clone());
                                                }
                                                GeminiPartResponse::FunctionCall {
                                                    function_call,
                                                } => {
                                                    let msg = Message::Assistant {
                                                        id: uuid::Uuid::new_v4(),
                                                        content: vec![ContentBlock::ToolUse {
                                                            id: uuid::Uuid::new_v4().to_string(),
                                                            name: function_call.name.clone(),
                                                            input: function_call.args.clone(),
                                                        }],
                                                        model: String::new(),
                                                        stop_reason: None,
                                                        usage: None,
                                                        timestamp: chrono::Utc::now(),
                                                    };
                                                    let _ = tx.send(Ok(msg)).await;
                                                }
                                            }
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            }

            // Flush remaining text
            if !text_parts.is_empty() {
                let text = text_parts.join("");
                let _ = tx
                    .send(Ok(Message::Assistant {
                        id: uuid::Uuid::new_v4(),
                        content: vec![ContentBlock::Text { text }],
                        model: String::new(),
                        stop_reason: Some("STOP".into()),
                        usage: None,
                        timestamp: chrono::Utc::now(),
                    }))
                    .await;
            }
        });

        Ok(Box::pin(tokio_stream::wrappers::ReceiverStream::new(rx)))
    }

    async fn supports_model(&self, _model: &str) -> bool {
        true
    }
}
