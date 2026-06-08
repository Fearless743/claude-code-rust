use std::sync::Arc;

#[cfg(feature = "bridge-mode")]
mod transport;

#[cfg(feature = "bridge-mode")]
pub async fn start_bridge(config: BridgeConfig) -> eyre::Result<()> {
    use tokio::sync::mpsc;

    let (session_tx, mut session_rx) = mpsc::unbounded_channel::<SessionEvent>();
    let (work_tx, mut work_rx) = mpsc::unbounded_channel::<WorkItem>();

    let api = Arc::new(BridgeApiClient::new(&config)?);
    let env_id = Arc::new(api.register_environment().await?);

    tracing::info!("Bridge registered as environment: {}", *env_id);

    let poll_api = api.clone();
    let poll_env_id = env_id.clone();
    let poll_handle = tokio::spawn(async move {
        loop {
            match poll_api.poll_work(&poll_env_id).await {
                Ok(Some(work)) => {
                    let _ = work_tx.send(work);
                }
                Ok(None) => {
                    tokio::time::sleep(std::time::Duration::from_secs(2)).await;
                }
                Err(e) => {
                    tracing::error!("Poll error: {e}");
                    tokio::time::sleep(std::time::Duration::from_secs(5)).await;
                }
            }
        }
    });

    while let Some(work) = work_rx.recv().await {
        let api = api.clone();
        let env_id = env_id.clone();
        let session_tx = session_tx.clone();
        let work_id = work.id.clone();

        tokio::spawn(async move {
            api.ack_work(&env_id, &work_id).await.ok();

            let settings = crate::config::ConfigManager::new()
                .map(|c| c.merged_settings())
                .unwrap_or_default();

            let engine = match crate::engine::QueryEngine::new(settings, None).await {
                Ok(e) => e,
                Err(e) => {
                    let _ = session_tx.send(SessionEvent::Error(format!("{e}")));
                    api.stop_work(&env_id, &work_id).await.ok();
                    return;
                }
            };

            match engine.run(Some(work.prompt)).await {
                Ok(messages) => {
                    for msg in messages {
                        let _ = session_tx.send(SessionEvent::Message(msg));
                    }
                    let _ = session_tx.send(SessionEvent::Completed);
                }
                Err(e) => {
                    let _ = session_tx.send(SessionEvent::Error(format!("{e}")));
                }
            }

            let _ = api.heartbeat(&env_id, &work_id).await;
            api.stop_work(&env_id, &work_id).await.ok();
        });
    }

    let _ = api.deregister(&env_id).await;
    poll_handle.abort();
    Ok(())
}

#[derive(Debug, Clone)]
pub struct BridgeConfig {
    pub bridge_url: String,
    pub auth_token: Option<String>,
    pub environment_name: String,
}

impl Default for BridgeConfig {
    fn default() -> Self {
        Self {
            bridge_url: std::env::var("CLAUDE_BRIDGE_URL")
                .unwrap_or_else(|_| "https://bridge.anthropic.com".into()),
            auth_token: std::env::var("CLAUDE_BRIDGE_TOKEN").ok(),
            environment_name: hostname(),
        }
    }
}

#[derive(Debug)]
pub enum SessionEvent {
    Message(crate::api::message::Message),
    Completed,
    Error(String),
}

#[derive(Debug, Clone)]
pub struct WorkItem {
    pub id: String,
    pub prompt: String,
    pub session_id: Option<String>,
}

#[derive(Clone)]
struct BridgeApiClient {
    base_url: String,
    token: Option<String>,
    client: reqwest::Client,
}

impl BridgeApiClient {
    fn new(config: &BridgeConfig) -> eyre::Result<Self> {
        Ok(Self {
            base_url: config.bridge_url.trim_end_matches('/').to_string(),
            token: config.auth_token.clone(),
            client: reqwest::Client::new(),
        })
    }

    async fn register_environment(&self) -> eyre::Result<String> {
        let resp = self
            .client
            .post(format!("{}/v1/environments/bridge", self.base_url))
            .bearer_auth(self.token.as_deref().unwrap_or(""))
            .json(&serde_json::json!({"name": "claude-code-rust", "version": env!("CARGO_PKG_VERSION")}))
            .send()
            .await?;
        let body: serde_json::Value = resp.json().await?;
        body["id"]
            .as_str()
            .map(|s| s.to_string())
            .ok_or_else(|| eyre::eyre!("No environment ID in response"))
    }

    async fn poll_work(&self, env_id: &str) -> eyre::Result<Option<WorkItem>> {
        let resp = self
            .client
            .get(format!(
                "{}/v1/environments/{env_id}/work/poll",
                self.base_url
            ))
            .bearer_auth(self.token.as_deref().unwrap_or(""))
            .send()
            .await?;

        if resp.status() == 204 {
            return Ok(None);
        }

        let body: serde_json::Value = resp.json().await?;
        Ok(Some(WorkItem {
            id: body["id"].as_str().unwrap_or_default().into(),
            prompt: body["prompt"].as_str().unwrap_or_default().into(),
            session_id: body["sessionId"].as_str().map(|s| s.into()),
        }))
    }

    async fn ack_work(&self, env_id: &str, work_id: &str) -> eyre::Result<()> {
        self.client
            .post(format!(
                "{}/v1/environments/{env_id}/work/{work_id}/ack",
                self.base_url
            ))
            .bearer_auth(self.token.as_deref().unwrap_or(""))
            .send()
            .await?;
        Ok(())
    }

    async fn stop_work(&self, env_id: &str, work_id: &str) -> eyre::Result<()> {
        self.client
            .post(format!(
                "{}/v1/environments/{env_id}/work/{work_id}/stop",
                self.base_url
            ))
            .bearer_auth(self.token.as_deref().unwrap_or(""))
            .send()
            .await?;
        Ok(())
    }

    async fn heartbeat(&self, env_id: &str, work_id: &str) -> eyre::Result<()> {
        self.client
            .post(format!(
                "{}/v1/environments/{env_id}/work/{work_id}/heartbeat",
                self.base_url
            ))
            .bearer_auth(self.token.as_deref().unwrap_or(""))
            .send()
            .await?;
        Ok(())
    }

    async fn deregister(&self, env_id: &str) -> eyre::Result<()> {
        self.client
            .delete(format!("{}/v1/environments/bridge/{env_id}", self.base_url))
            .bearer_auth(self.token.as_deref().unwrap_or(""))
            .send()
            .await?;
        Ok(())
    }
}

fn hostname() -> String {
    std::env::var("HOSTNAME")
        .or_else(|_| std::env::var("HOST"))
        .unwrap_or_else(|_| "unknown".into())
}
