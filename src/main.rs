mod acp;
mod api;
mod auth;
mod bridge;
mod cli;
mod commands;
mod config;
mod context;
mod daemon;
mod engine;
mod mcp;
mod permissions;
mod providers;
mod state;
mod tools;
mod ui;
mod utils;

use std::process;

fn main() {
    let rt = tokio::runtime::Runtime::new().expect("Failed to create tokio runtime");
    if let Err(e) = rt.block_on(cli::run()) {
        eprintln!("Error: {e:#}");
        process::exit(1);
    }
}
