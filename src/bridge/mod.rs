#[cfg(feature = "bridge-mode")]
pub mod transport;

#[cfg(feature = "bridge-mode")]
pub async fn start_bridge() -> eyre::Result<()> {
    todo!("Bridge mode not yet implemented")
}
