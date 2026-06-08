# Handoff Prompt — 给下一个 AI 继续开发

**工作目录**: `/home/fearless/github/claude-code-rust/`
**GitHub**: https://github.com/Fearless743/claude-code-rust
**目标**: 将 Anthropic Claude Code CLI 从 TypeScript/Bun 重写为 Rust

---

## 环境验证（第一步）

```bash
cd /home/fearless/github/claude-code-rust
cargo check          # 类型检查，必须通过
cargo test           # 运行 12 个测试，必须全部通过
cargo fmt --check    # 格式检查
git log --oneline -5 # 查看历史
```

---

## 已完成的工作

| Phase | 内容 |
|-------|------|
| 0 | Cargo workspace + clap CLI scaffold |
| 1 | Anthropic SSE 流式客户端 + QueryEngine turn loop |
| 2 | 9 个内置工具 (Bash/Read/Write/Edit/Glob/Grep/WebFetch/WebSearch/Ask) |
| 3 | ratatui REPL (INSERT/NORMAL 模式) |
| 4 | MCP 协议完整实现 (types/stdio transport/McpClient/CLI) |
| 5 | OpenAI + Gemini + Grok 适配器 |
| 6 | Bridge/Daemon/ACP/会话持久化/上下文压缩 |

## 待完成: Phase 7

1. **集成测试** — mock HTTP server，测试完整 turn loop
2. **错误恢复** — SSE 流中断、tool crash、网络超时的优雅处理
3. **更多单元测试** — 每个 adapter、每个 MCP 方法、每个 tool 的边界情况
4. **Bedrock/Vertex/Foundry adapter** — 目前是 stub，需要实现
5. **性能优化** — 减少启动时间、优化 token 计算
6. **Clippy lint 清理** — `cargo clippy` 修复 warnings
7. **CI pipeline** — `.github/workflows/ci.yml` 已存在但需要验证

---

## 关键命令

```bash
cargo run                          # 交互 REPL
echo "hello" | cargo run -- -p     # Headless pipe mode
cargo run --features bridge-mode -- bridge
cargo run --features daemon -- daemon
CLAUDE_CODE_USE_OPENAI=1 OPENAI_API_KEY=sk-... cargo run
CLAUDE_CODE_USE_GEMINI=1 GEMINI_API_KEY=... cargo run
```

---

## 文件映射（找代码用）

| TS 源文件 | Rust 源文件 |
|-----------|------------|
| `src/services/api/claude.ts` | `src/api/anthropic.rs` |
| `src/query.ts` | `src/engine/query.rs` |
| `src/screens/REPL.tsx` | `src/ui/repl.rs` |
| `src/tools.ts` | `src/tools/mod.rs` |
| `packages/builtin-tools/src/tools/` | `src/tools/*.rs` |
| `src/bridge/` | `src/bridge/` |
| `src/services/acp/` | `src/acp/` |

---

## Git 提交规范

使用 Conventional Commits，每次修改后提交：

```bash
git add -A && git commit -m "feat: description" && git push origin main
```

Type: `feat`, `fix`, `chore`, `refactor`, `test`, `docs`
