#[cfg(feature = "daemon")]
mod worker;

#[cfg(feature = "daemon")]
use std::process::{Child, Command};
use std::sync::Arc;
use tokio::sync::Mutex;
use tokio::time::{Duration, sleep};

#[cfg(feature = "daemon")]
pub struct DaemonSupervisor {
    workers: Arc<Mutex<Vec<WorkerProcess>>>,
}

#[cfg(feature = "daemon")]
struct WorkerProcess {
    process: Child,
    kind: WorkerKind,
    restart_count: u32,
    last_start: std::time::Instant,
}

#[cfg(feature = "daemon")]
#[derive(Debug, Clone)]
enum WorkerKind {
    RemoteControl,
    Bridge,
}

#[cfg(feature = "daemon")]
impl DaemonSupervisor {
    pub fn new() -> Self {
        Self {
            workers: Arc::new(Mutex::new(Vec::new())),
        }
    }

    pub async fn start_worker(&self, kind: WorkerKind) -> eyre::Result<()> {
        let (cmd, args) = match &kind {
            WorkerKind::RemoteControl => ("claude", vec!["remote-control".to_string()]),
            WorkerKind::Bridge => ("claude", vec!["bridge".to_string()]),
        };

        let process = Command::new(cmd)
            .args(&args)
            .spawn()
            .map_err(|e| eyre::eyre!("Failed to spawn worker: {e}"))?;

        self.workers.lock().await.push(WorkerProcess {
            process,
            kind,
            restart_count: 0,
            last_start: std::time::Instant::now(),
        });

        Ok(())
    }

    pub async fn supervise_loop(&self) {
        loop {
            sleep(Duration::from_secs(5)).await;
            let mut workers = self.workers.lock().await;

            let mut dead_indices = Vec::new();
            for (i, worker) in workers.iter_mut().enumerate() {
                match worker.process.try_wait() {
                    Ok(Some(status)) => {
                        tracing::warn!("Worker {:?} exited with {status}", worker.kind);
                        if worker.restart_count < 5
                            && worker.last_start.elapsed() > Duration::from_secs(2)
                        {
                            tracing::info!("Restarting worker {:?}", worker.kind);
                            if let Ok(new_proc) = self.spawn_worker_process(&worker.kind) {
                                worker.process = new_proc;
                                worker.restart_count += 1;
                                worker.last_start = std::time::Instant::now();
                            }
                        } else {
                            dead_indices.push(i);
                        }
                    }
                    Ok(None) => {}
                    Err(e) => {
                        tracing::error!("Error checking worker: {e}");
                    }
                }
            }

            // Remove permanently dead workers
            for i in dead_indices.into_iter().rev() {
                workers.remove(i);
            }
        }
    }

    fn spawn_worker_process(&self, kind: &WorkerKind) -> eyre::Result<Child> {
        let (cmd, args) = match kind {
            WorkerKind::RemoteControl => ("claude", vec!["remote-control".to_string()]),
            WorkerKind::Bridge => ("claude", vec!["bridge".to_string()]),
        };
        Command::new(cmd)
            .args(&args)
            .spawn()
            .map_err(|e| eyre::eyre!("Failed to spawn: {e}"))
    }

    pub async fn shutdown(&self) {
        let mut workers = self.workers.lock().await;
        for worker in workers.iter_mut() {
            let _ = worker.process.kill();
        }
        workers.clear();
        tracing::info!("All workers stopped");
    }
}

#[cfg(feature = "daemon")]
pub async fn start_daemon() -> eyre::Result<()> {
    let supervisor = DaemonSupervisor::new();
    supervisor.start_worker(WorkerKind::RemoteControl).await?;
    supervisor.supervise_loop().await;
    Ok(())
}

#[cfg(not(feature = "daemon"))]
pub async fn start_daemon() -> eyre::Result<()> {
    eyre::bail!("Daemon feature not enabled. Build with --features daemon")
}
