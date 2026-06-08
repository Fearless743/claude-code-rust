mod commands;
mod options;

use clap::{Parser, Subcommand};
use eyre::Result;

#[derive(Parser)]
#[command(name = "claude", version = env!("CARGO_PKG_VERSION"), about = "Claude Code CLI")]
pub struct Cli {
    #[command(subcommand)]
    pub command: Option<Commands>,

    #[arg(short = 'p', long = "print")]
    pub print: bool,

    #[arg(long = "dangerously-skip-permissions")]
    pub skip_permissions: bool,

    #[arg(short = 'c', long = "continue")]
    pub r#continue: bool,

    #[arg(long = "model")]
    pub model: Option<String>,

    #[arg(short = 'd', long = "debug")]
    pub debug: bool,

    pub prompt: Vec<String>,
}

#[derive(Subcommand)]
pub enum Commands {
    Mcp {
        #[command(subcommand)]
        cmd: McpCommand,
    },
    Auth {
        #[command(subcommand)]
        cmd: AuthCommand,
    },
    Update,
    Doctor,
    Config,
}

#[derive(Subcommand, Clone)]
pub enum McpCommand {
    Add {
        name: String,
        command: String,
        args: Vec<String>,
    },
    Remove {
        name: String,
    },
    List,
    Get {
        name: String,
    },
}

#[derive(Subcommand, Clone)]
pub enum AuthCommand {
    Login,
    Logout,
    Status,
}

pub async fn run() -> Result<()> {
    tracing_subscriber::fmt::init();
    let cli = Cli::parse();
    let prompt = cli.prompt.join(" ");

    match &cli.command {
        Some(Commands::Mcp { cmd }) => commands::mcp::handle(cmd).await?,
        Some(Commands::Auth { cmd }) => commands::auth::handle(cmd).await?,
        Some(Commands::Update) => commands::update::handle().await?,
        Some(Commands::Doctor) => commands::doctor::handle().await?,
        Some(Commands::Config) => commands::config_cmd::handle().await?,
        None => {
            if cli.print {
                run_headless(cli, prompt).await?;
            } else {
                run_interactive(cli, prompt)?;
            }
        }
    }

    Ok(())
}

async fn run_headless(cli: Cli, prompt: String) -> Result<()> {
    let config = crate::config::ConfigManager::new()?;
    let mut settings = config.merged_settings();

    if let Some(model) = &cli.model {
        settings.model = Some(model.clone());
    }

    if settings.resolve_api_key().is_none() {
        eprintln!("Error: No API key. Set ANTHROPIC_API_KEY or run 'claude auth login'");
        std::process::exit(1);
    }

    let prompt = if prompt.is_empty() {
        let mut input = String::new();
        std::io::Read::read_to_string(&mut std::io::stdin(), &mut input)?;
        input.trim().to_string()
    } else {
        prompt
    };

    if prompt.is_empty() {
        eprintln!("Error: No input provided");
        std::process::exit(1);
    }

    eprintln!("Running headless: {prompt}");
    eprintln!("Model: {}", settings.resolve_model());
    eprintln!("---");

    let engine = crate::engine::QueryEngine::new(settings, None).await?;
    let messages = engine.run(Some(prompt)).await?;

    for msg in messages.iter().rev() {
        if let crate::api::message::Message::Assistant { content, .. } = msg {
            for block in content {
                if let crate::api::message::ContentBlock::Text { text } = block {
                    println!("{text}");
                }
            }
            break;
        }
    }

    Ok(())
}

fn run_interactive(cli: Cli, prompt: String) -> Result<()> {
    let config = crate::config::ConfigManager::new()?;
    let mut settings = config.merged_settings();

    if let Some(model) = &cli.model {
        settings.model = Some(model.clone());
    }

    if settings.resolve_api_key().is_none() {
        println!("No API key configured. Set ANTHROPIC_API_KEY or run 'claude auth login'");
        println!("Entering REPL without API — type /help for commands, /quit to exit");
    }

    let initial_prompt = if prompt.is_empty() {
        None
    } else {
        Some(prompt)
    };
    crate::ui::repl::run_repl(settings, initial_prompt)?;

    Ok(())
}
