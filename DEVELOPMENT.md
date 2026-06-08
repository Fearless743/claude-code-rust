# Claude Code Rust — Development Guide

## Quick Start

```bash
cargo build
cargo run
```

## Implementation Status

- [x] Phase 0: Cargo workspace scaffold
- [x] Phase 1: Anthropic SSE streaming + QueryEngine turn loop
- [x] Phase 2: 9 built-in tools (Bash, FileRead/Write/Edit, Glob, Grep, WebFetch, WebSearch, Ask)
- [x] Phase 3: ratatui REPL (INSERT/NORMAL modes)
- [x] Phase 4: MCP protocol (JSON-RPC types, stdio transport, McpClient)
- [x] Phase 5: OpenAI/Gemini/Grok/Bedrock/Vertex/Foundry adapters (OpenAI+Gemini+Grok implemented)
- [x] Phase 6: Bridge mode, Daemon supervisor, ACP agent, Session persistence, Context compaction
- [ ] Phase 7: Comprehensive tests, edge cases, error recovery, CI

## Next Steps for Phase 7

1. **Integration tests**: Test full turn loop end-to-end with mocked HTTP
2. **Error recovery**: Handle network failures, partial SSE streams, tool crashes
3. **More unit tests**: Each tool, each adapter, each MCP method
4. **CI/CD**: GitHub Actions workflow already exists (`.github/workflows/ci.yml`)
5. **Documentation**: Add `cargo doc` docstrings to public API
6. **Performance**: Profile and optimize startup time

## Code Conventions

- Use `eyre::Result` for application code, `thiserror` for library errors
- `#[async_trait]` for async trait methods
- `serde(rename_all = "camelCase")` for JSON API types
- Follow Rust 2024 edition idioms
- Tests in `#[cfg(test)] mod tests` at bottom of each file

## File Map (for AI agents)

| TS Source | Rust Source | Notes |
|-----------|-------------|-------|
| `src/services/api/claude.ts` | `src/api/anthropic.rs` | Core SSE streaming |
| `src/services/api/openai/` | `src/api/openai.rs` | OpenAI adapter |
| `src/services/api/gemini/` | `src/api/gemini.rs` | Gemini adapter |
| `src/query.ts` | `src/engine/query.rs` | Turn loop |
| `src/QueryEngine.ts` | `src/engine/mod.rs` | High-level orchestrator |
| `src/screens/REPL.tsx` | `src/ui/repl.rs` | Terminal UI |
| `src/Tool.ts` / `src/tools.ts` | `src/tools/mod.rs` | Tool system |
| `packages/builtin-tools/src/tools/` | `src/tools/*.rs` | Individual tools |
| `src/bridge/` | `src/bridge/` | Remote control |
| `src/daemon/` | `src/daemon/` | Supervisor |
| `src/services/acp/` | `src/acp/` | ACP protocol |
| `src/context.ts` | `src/context/` | CLAUDE.md + git |
| `src/utils/claudemd.ts` | `src/context/claude_md.rs` | CLAUDE.md loader |
