# Claude Code CLI (Rust Rewrite)

A Rust rewrite of the Claude Code CLI tool. Uses tokio, ratatui, clap, and reqwest.

## Building

```bash
cargo build --release
```

## Running

```bash
# Interactive mode
cargo run

# Headless mode
echo "say hello" | cargo run -- -p

# Show version
cargo run -- --version
```

## Development

```bash
cargo check --all-features
cargo test --all-features
cargo clippy --all-features -- -D warnings
cargo fmt --check
```
