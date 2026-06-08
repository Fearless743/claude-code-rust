use crate::cli::McpCommand;
use eyre::Result;

pub async fn handle(cmd: &McpCommand) -> Result<()> {
    match cmd {
        McpCommand::Add {
            name,
            command,
            args,
        } => {
            println!("Adding MCP server '{name}'...");
            println!("  Command: {command} {}", args.join(" "));
            match crate::mcp::transport::McpClient::connect(command, args) {
                Ok(mut client) => {
                    println!("  Connected: {:?}", client.server_info());
                    match client.list_tools() {
                        Ok(tools) => {
                            println!("  {} tools discovered:", tools.len());
                            for tool in &tools {
                                println!("    - {}: {}", tool.name, tool.description);
                            }
                        }
                        Err(e) => eprintln!("  Warning: {e}"),
                    }
                }
                Err(e) => eprintln!("  Error: {e}"),
            }
        }
        McpCommand::Remove { name } => println!("Removing MCP server '{name}'"),
        McpCommand::List => println!("MCP servers: (none configured)"),
        McpCommand::Get { name } => println!("MCP server '{name}': not found"),
    }
    Ok(())
}
