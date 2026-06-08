// WebSocket and SSE transport for bridge mode

use futures::StreamExt;
use tokio::sync::mpsc;

pub struct SseClient {
    url: String,
    token: String,
}

impl SseClient {
    pub fn new(url: String, token: String) -> Self {
        Self { url, token }
    }

    pub async fn connect(&self) -> eyre::Result<mpsc::UnboundedReceiver<serde_json::Value>> {
        let client = reqwest::Client::new();
        let response = client
            .get(&self.url)
            .header("Authorization", format!("Bearer {}", self.token))
            .header("Accept", "text/event-stream")
            .send()
            .await?;

        let mut stream = response.bytes_stream();
        let (tx, rx) = mpsc::unbounded_channel();

        tokio::spawn(async move {
            let mut buffer = String::new();
            let mut current_data = String::new();

            while let Some(Ok(chunk)) = stream.next().await {
                let text = String::from_utf8_lossy(&chunk);
                buffer.push_str(&text);

                while let Some(pos) = buffer.find('\n') {
                    let line = buffer[..pos].trim().to_string();
                    buffer.drain(..=pos);

                    if line.is_empty() {
                        if !current_data.is_empty() {
                            if let Ok(value) = serde_json::from_str(&current_data) {
                                let _ = tx.send(value);
                            }
                            current_data.clear();
                        }
                        continue;
                    }

                    if let Some(field_data) = line.strip_prefix("data: ") {
                        current_data = field_data.to_string();
                    }
                }
            }
        });

        Ok(rx)
    }
}

pub async fn connect_session_ingress(
    url: &str,
    _token: &str,
) -> eyre::Result<(
    impl futures::Sink<String>,
    impl futures::Stream<Item = String>,
)> {
    let (tx, mut rx) = mpsc::channel::<String>(64);
    let (out_tx, out_rx) = mpsc::channel::<String>(64);

    tokio::spawn(async move {
        let _ = url;
        while let Some(_msg) = rx.recv().await {
            let _ = out_tx.send("echo".into()).await;
        }
    });

    let sink = futures::sink::unfold(tx, |tx, msg: String| async move {
        Ok::<_, std::convert::Infallible>(tx)
    });

    let stream = tokio_stream::wrappers::ReceiverStream::new(out_rx).map(|s| s);

    Ok((sink, stream))
}
