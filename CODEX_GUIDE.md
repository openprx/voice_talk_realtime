# Codex 使用指南 (QA 环境)

## Codex 位置
```bash
npx codex  # 推荐方式
# 或完整路径:
# /home/xx/.npm-global/lib/node_modules/@openai/codex/node_modules/@openai/codex-linux-x64/vendor/x86_64-unknown-linux-musl/codex/codex
```

## 基本用法

### 交互模式（需要 TTY）
```bash
cd /opt/code/voice_talk_realtime
npx codex
```
进入后直接输入任务描述。

### 非交互模式（推荐用于自动化）
```bash
cd /opt/code/voice_talk_realtime
npx codex exec --full-auto "任务描述"
```

### 关键参数
| 参数 | 说明 |
|------|------|
| `exec` | 非交互执行模式 |
| `--full-auto` | 跳过确认，自动执行所有操作 |
| `-w /path` | 指定工作目录 |
| `--model gpt-5.4` | 指定模型（默认用配置的模型） |

## 在 OpenPRX shell 工具中调用 Codex

Vano 可以通过 shell 工具调用 Codex：

```bash
# Step 1: 确保环境变量
source ~/.cargo/env
export PATH=$PATH:$HOME/.npm-global/bin

# Step 2: 执行任务
cd /opt/code/voice_talk_realtime
npx codex exec --full-auto "Create Cargo.toml for a WASM project with wasm-bindgen, web-sys, js-sys dependencies. Target: wasm32-unknown-unknown."
```

## 任务拆分原则

1. **一次一件事** — 每次 codex exec 只做一个明确的任务
2. **给足上下文** — 在 prompt 中说清楚现有文件结构、要改什么、期望结果
3. **验证后再下一步** — 每次 codex 完成后检查输出，确认正确再继续
4. **不要太大的任务** — 拆成 Phase，每个 Phase 一次 codex 调用

## 示例：Phase 1 执行流程

```bash
# 1. 安装 WASM 工具链
source ~/.cargo/env
rustup target add wasm32-unknown-unknown
cargo install wasm-pack

# 2. 创建项目骨架
cd /opt/code/voice_talk_realtime
npx codex exec --full-auto "
Initialize a Rust WASM project for real-time voice conversation.
Create:
- Cargo.toml with dependencies: wasm-bindgen, web-sys (features: WebSocket, AudioContext, etc.), js-sys, serde, serde_json, base64
- src/lib.rs with basic WASM entry point
- src/realtime/mod.rs with trait RealtimeClient { connect, send_text, send_audio, on_event }
- src/realtime/protocol.rs with event types (SessionCreate, AudioBufferAppend, ResponseDelta, etc.)
Target: wasm32-unknown-unknown. Must pass cargo check --target wasm32-unknown-unknown.
"

# 3. 验证
cargo check --target wasm32-unknown-unknown

# 4. 下一个任务...
npx codex exec --full-auto "
Implement OpenAI Realtime API WebSocket client in src/realtime/openai.rs.
Use web-sys WebSocket. Connect to wss://api.openai.com/v1/realtime.
Handle events: session.created, response.text.delta, response.audio.delta, error.
Send events: session.update, input_audio_buffer.append, response.create.
Must compile for wasm32-unknown-unknown.
"
```

## 注意事项

1. **Codex 用的是 codex OAuth token**（~/.codex/auth.json），不需要额外 API key
2. **每次调用有 token 消耗**，不要无意义重复
3. **Codex 会直接修改文件**，full-auto 模式下不会确认
4. **如果 Codex 报错**，把错误信息包含在下一次 prompt 中让它修
5. **cargo check 是最基本的验证**，每次改完都跑一次
