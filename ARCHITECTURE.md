# Claude Code Rust — Architecture

Rust rewrite of Anthropic's Claude Code CLI tool, targeting feature-parity.

## Project Structure

```
src/
├── main.rs              # Entry point, clap CLI dispatch
├── api/                 # API providers (Anthropic, OpenAI, Gemini, Grok, Bedrock, Vertex)
│   ├── mod.rs           # Provider trait + factory
│   ├── anthropic.rs     # Anthropic SSE streaming (primary)
│   ├── openai.rs        # OpenAI Chat Completions adapter
│   ├── gemini.rs        # Gemini streamGenerateContent adapter
│   ├── grok.rs          # Grok (xAI) via OpenAI protocol
│   ├── bedrock.rs       # AWS Bedrock (stub)
│   ├── vertex.rs        # GCP Vertex (stub)
│   ├── foundry.rs       # Foundry (stub)
│   ├── message.rs       # Message/ContentBlock/Usage types
│   └── types.rs         # Shared API types
├── auth/                # Authentication
├── bridge/              # Remote Control Bridge (feature: bridge-mode)
│   ├── mod.rs           # Bridge loop: poll → work → ack → stop
│   └── transport.rs     # SSE + WebSocket primitives
├── cli/                 # CLI subcommands (auth, config, doctor, mcp, update)
├── config/              # Settings (.claude/settings.json + env vars)
├── context/             # CLAUDE.md loader, git context
├── daemon/              # Daemon supervisor (feature: daemon)
│   ├── mod.rs           # Process supervisor with auto-restart
│   └── worker.rs        # Worker entry point
├── acp/                 # Agent Client Protocol (stdin/stdout JSON-RPC)
│   ├── mod.rs
│   ├── agent.rs         # ACP request/response types + reader/writer
│   └── bridge.rs        # ACP ↔ Claude message conversion
├── engine/              # Core query loop
│   ├── mod.rs
│   ├── query.rs         # QueryEngine: turn loop, tool dispatch
│   ├── session.rs       # Session persistence (save/load ~/.claude/sessions/)
│   └── compaction.rs    # Context compaction (token budget)
├── mcp/                 # Model Context Protocol
│   ├── mod.rs
│   ├── types.rs         # JSON-RPC types + MCP tool defs
│   ├── transport.rs     # Stdio transport (spawn process)
│   └── client.rs        # McpClient (connect, list_tools, call_tool)
├── permissions/         # Tool permission modes + rules
├── providers/           # Provider selection logic
├── tools/               # 9 built-in tools
│   ├── mod.rs           # Tool trait + ToolRegistry
│   ├── bash.rs          # Shell execution
│   ├── file_read.rs     # File reading
│   ├── file_write.rs    # File writing
│   ├── file_edit.rs     # Search-and-replace editing
│   ├── glob.rs          # Glob pattern matching
│   ├── grep.rs          # Regex search
│   ├── web_fetch.rs     # HTTP GET with content extraction
│   ├── web_search.rs    # Web search (placeholder)
│   ├── ask.rs           # User question tool
│   ├── task.rs          # Agent task delegation
│   └── dispatch.rs      # Tool execution dispatcher
├── ui/                  # ratatui REPL
│   ├── repl.rs          # Main REPL (INSERT/NORMAL modes)
│   ├── messages.rs      # Message rendering
│   ├── prompt.rs        # Input handling
│   ├── diff.rs          # Diff display
│   ├── permissions.rs   # Permission prompts
│   ├── spinner.rs       # Loading spinner
│   └── themes.rs        # Color themes
└── utils/               # Utilities (env, paths, platform, token counting)
```

## Data Flow

```
User Input → REPL (ratatui) → QueryEngine → Provider.stream_completion()
                                                    ↓
                                           SSE Stream Parsing
                                                    ↓
                                          Message → ToolRegistry.dispatch()
                                                    ↓
                                          Tool.execute() → Result → next turn
```

## Provider Architecture

All providers implement the `Provider` trait:

```rust
#[async_trait]
pub trait Provider: Send + Sync {
    async fn stream_completion(
        &self, messages, system_prompt, tools, config
    ) -> Result<Pin<Box<dyn Stream<Item = Result<Message, ApiError>> + Send>>, ApiError>;
}
```

Each adapter converts Claude-format messages to the target API format and streams back. The factory `get_provider()` selects based on `settings.resolve_provider()` or env vars:

- `CLAUDE_CODE_USE_OPENAI=1` → OpenAI adapter
- `CLAUDE_CODE_USE_GEMINI=1` → Gemini adapter
- `CLAUDE_CODE_USE_GROK=1` → Grok adapter
- Default → Anthropic SSE

## Key Dependencies

| Crate | Purpose |
|-------|---------|
| tokio | Async runtime |
| clap | CLI argument parsing |
| ratatui | Terminal UI |
| reqwest | HTTP client |
| serde / serde_json | Serialization |
| eyre / thiserror | Error handling |
| uuid | Message/session IDs |
| chrono | Timestamps |
| tokio-tungstenite | WebSocket (bridge mode) |
| tokio-stream | Stream adapters |

## Running

```bash
# Interactive REPL
cargo run

# Headless (pipe mode)
echo "fix the bug in main.rs" | cargo run -- -p

# Specify provider
CLAUDE_CODE_USE_OPENAI=1 OPENAI_API_KEY=sk-... cargo run
CLAUDE_CODE_USE_GEMINI=1 GEMINI_API_KEY=... cargo run

# Bridge mode
cargo run --features bridge-mode -- bridge

# Daemon mode
cargo run --features daemon -- daemon
```

## Test Coverage

```bash
cargo test          # 12 tests (session × 3, compaction × 2, tool × 4, api × 3)
cargo check         # Type checking
cargo fmt --check   # Format validation
```

## Feature Flags

- `bridge-mode`: Remote control bridge (poll/ack/heartbeat loop)
- `daemon`: Process supervisor (auto-restart workers)
