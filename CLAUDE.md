# CLAUDE.md

This file provides guidance to Claude Code and other AI coding agents when working with this Rust rewrite of the Claude Code CLI.

## Project Overview

Rust rewrite of Anthropic's Claude Code CLI. Full architecture in `ARCHITECTURE.md`.

## Commands

```bash
cargo build                    # Build
cargo run                      # Interactive REPL
echo "hello" | cargo run -- -p # Headless/pipe mode
cargo test                     # Run all 12 tests
cargo test -- --nocapture      # Run tests with output
cargo check                    # Type checking (fast)
cargo fmt                      # Format code
cargo clippy                   # Lint
```

## Code Style

- Use `thiserror` for library error types, `eyre::Result` for application code
- Inline tests at bottom of each `.rs` file: `#[cfg(test)] mod tests { ... }`
- Prefer `Arc<str>` over `Arc<String>`, `&str` over `&String` in signatures
- Keep functions under 50 lines when reasonable
- Use `serde(rename_all = "camelCase")` for JSON-facing types

## Implementation Status

See `DEVELOPMENT.md` for full phase tracking. Currently at Phase 7 (tests & polish).

## Git Convention

Use Conventional Commits: `feat:`, `fix:`, `chore:`, `refactor:`
