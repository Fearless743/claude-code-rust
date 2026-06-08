use async_trait::async_trait;
use std::pin::Pin;

use super::message::Message;
use super::{ApiError, Provider, RequestConfig, ToolDef};
use crate::config::Settings;

pub struct GrokProvider {
    openai_provider: super::openai::OpenaiProvider,
}

impl GrokProvider {
    pub fn new(settings: &Settings) -> Result<Self, ApiError> {
        let mut openai_settings = settings.clone();
        openai_settings.api_key = settings
            .api_key
            .clone()
            .or_else(|| std::env::var("GROK_API_KEY").ok());
        let provider = super::openai::OpenaiProvider::new(&openai_settings)?;
        Ok(Self {
            openai_provider: provider,
        })
    }
}

#[async_trait]
impl Provider for GrokProvider {
    async fn stream_completion(
        &self,
        messages: Vec<Message>,
        system_prompt: &str,
        tools: &[ToolDef],
        config: &RequestConfig,
    ) -> Result<Pin<Box<dyn futures::Stream<Item = Result<Message, ApiError>> + Send>>, ApiError>
    {
        self.openai_provider
            .stream_completion(messages, system_prompt, tools, config)
            .await
    }

    async fn supports_model(&self, model: &str) -> bool {
        self.openai_provider.supports_model(model).await
    }
}
