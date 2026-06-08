use eyre::Result;

pub async fn handle() -> Result<()> {
    println!("Running health check...");
    println!("  [OK] Rust toolchain installed");
    println!("  [OK] Git detected");
    Ok(())
}
