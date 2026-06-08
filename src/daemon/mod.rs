#[cfg(feature = "daemon")]
pub mod worker;

#[cfg(feature = "daemon")]
pub async fn start_daemon() -> eyre::Result<()> {
    todo!("Daemon mode not yet implemented")
}
