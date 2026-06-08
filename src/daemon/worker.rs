// Worker entry point for daemon mode
// Spawned by the supervisor, runs independently

pub async fn run_worker(kind: &str) -> eyre::Result<()> {
    match kind {
        "remoteControl" | "remote-control" => {
            tracing::info!("Starting remote control worker");
            // This would launch the bridge in headless mode
            crate::bridge::start_bridge(Default::default()).await
        }
        "bridge" => {
            tracing::info!("Starting bridge worker");
            crate::bridge::start_bridge(Default::default()).await
        }
        _ => eyre::bail!("Unknown worker kind: {kind}"),
    }
}
