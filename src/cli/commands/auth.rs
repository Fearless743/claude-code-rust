use crate::cli::AuthCommand;
use eyre::Result;

pub async fn handle(cmd: &AuthCommand) -> Result<()> {
    match cmd {
        AuthCommand::Login => println!("Launching OAuth login..."),
        AuthCommand::Logout => println!("Logging out..."),
        AuthCommand::Status => println!("Checking auth status..."),
    }
    Ok(())
}
