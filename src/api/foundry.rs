use super::message::Message;
use super::{ApiError, Provider, RequestConfig, ToolDef};
use crate::config::Settings;
use async_trait::async_trait;
use std::pin::Pin;

pub struct FoundryProvider {
    _api_key: String,
    _base_url: String,
}

impl FoundryProvider {
    pub fn new(settings: &Settings) -> Result<Self, ApiError> {
        Ok(Self {
            _api_key: settings.resolve_api_key().unwrap_or_default(),
            _base_url: settings.resolve_base_url(),
        })
    }
}

#[async_trait]
impl Provider for FoundryProvider {
    async fn stream_completion(
        &self,
        _messages: Vec<Message>,
        _system_prompt: &str,
        _tools: &[ToolDef],
        _config: &RequestConfig,
    ) -> Result<Pin<Box<dyn futures::Stream<Item = Result<Message, ApiError>> + Send>>, ApiError>
    {
        Err(ApiError::Auth(
            "Foundry provider not yet implemented".into(),
        ))
    }
    async fn supports_model(&self, _model: &str) -> bool {
        false
    }
}
