use eyre::Result;

pub async fn handle() -> Result<()> {
    println!("Configuration:");
    println!("  Config dir: ~/.claude/");
    Ok(())
}
