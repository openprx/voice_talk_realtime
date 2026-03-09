# voice_talk_realtime — Project Plan

## Overview
A Rust WASM plugin that provides real-time voice conversation capability, supporting both OpenAI and xAI Realtime API protocols. Ships with a built-in web demo.

## Architecture

```
voice_talk_realtime/
├── Cargo.toml
├── README.md
├── src/
│   ├── lib.rs              # WASM plugin entry, exports for OpenPRX plugin system
│   ├── realtime/
│   │   ├── mod.rs           # Shared traits and types
│   │   ├── openai.rs        # OpenAI Realtime API client (WebSocket)
│   │   ├── xai.rs           # xAI Realtime API client (WebSocket)
│   │   └── protocol.rs      # Common event/message protocol types
│   ├── audio/
│   │   ├── mod.rs
│   │   ├── capture.rs       # Web Audio API mic capture via JS interop
│   │   ├── playback.rs      # Audio playback via Web Audio API
│   │   └── codec.rs         # PCM16/Opus encoding helpers
│   └── web/
│       ├── mod.rs
│       ├── app.rs            # Web demo Yew/Leptos component
│       └── ui.rs             # UI state management
├── web-demo/
│   ├── index.html            # Demo entry point
│   ├── style.css
│   └── js/
│       └── audio_worklet.js  # AudioWorklet for mic capture
├── tests/
│   └── integration.rs
└── .cargo/
    └── config.toml           # WASM target config
```

## Key Design Decisions

### 1. Realtime API Protocol
Both OpenAI and xAI use WebSocket-based Realtime APIs:
- **OpenAI**: `wss://api.openai.com/v1/realtime?model=gpt-4o-realtime-preview`
- **xAI**: `wss://api.x.ai/v1/realtime?model=grok-3-fast` (similar protocol)

Both use JSON events over WebSocket with audio as base64-encoded PCM16/Opus chunks.

### 2. WASM Plugin Interface
Exports for OpenPRX WASM plugin system:
- `init(config: &str) -> Result<()>` — Initialize with provider config
- `start_session(provider: &str, model: &str) -> Result<SessionId>`
- `send_audio(session: SessionId, chunk: &[u8]) -> Result<()>`
- `receive_event(session: SessionId) -> Result<Option<Event>>`
- `close_session(session: SessionId) -> Result<()>`

### 3. Audio Pipeline
```
Mic → AudioWorklet → PCM16 chunks → WASM → base64 → WebSocket → API
API → WebSocket → base64 → WASM → PCM16 → AudioWorklet → Speaker
```

### 4. Web Demo
Minimal but functional:
- Provider selector (OpenAI / xAI)
- Model selector
- API key input (stored in localStorage, never sent to our server)
- Push-to-talk / Voice Activity Detection toggle
- Audio visualizer (waveform)
- Conversation transcript panel
- Connection status indicator

## Development Phases

### Phase 1: Foundation (Codex Task 1)
- Project scaffolding (Cargo.toml with wasm-bindgen, web-sys, js-sys)
- Install wasm-pack target: `rustup target add wasm32-unknown-unknown`
- Core protocol types (Event, Session, AudioFormat)
- OpenAI Realtime API WebSocket client
- Basic send/receive text messages (no audio yet)
- `cargo check --target wasm32-unknown-unknown` must pass

### Phase 2: xAI + Audio (Codex Task 2)
- xAI Realtime API client (adapt OpenAI client)
- Audio capture via Web Audio API + AudioWorklet
- Audio playback pipeline
- PCM16 base64 encode/decode
- End-to-end: mic → API → speaker

### Phase 3: Web Demo (Codex Task 3)
- HTML/CSS/JS web demo (vanilla JS, no framework — keep WASM simple)
- Provider/model selector
- Push-to-talk button
- Audio visualizer
- Transcript display
- Build with wasm-pack, serve with simple HTTP

### Phase 4: WASM Plugin Integration (Codex Task 4)
- OpenPRX WASM plugin exports
- SKILL.toml for skill discovery
- Plugin config schema
- Integration tests

## Tech Stack
- **Language**: Rust
- **WASM tooling**: wasm-pack + wasm-bindgen
- **WebSocket**: web-sys WebSocket API (browser-native)
- **Audio**: Web Audio API via web-sys
- **Web demo**: Vanilla HTML/JS (wasm-bindgen JS glue)
- **No server needed**: Everything runs in browser, connects directly to API providers

## Prerequisites on QA
```bash
source ~/.cargo/env
rustup target add wasm32-unknown-unknown
cargo install wasm-pack
```

## API Reference

### OpenAI Realtime API
- Endpoint: `wss://api.openai.com/v1/realtime?model={model}`
- Auth: `Authorization: Bearer {api_key}` (via Sec-WebSocket-Protocol or initial message)
- Events: session.create, input_audio_buffer.append, response.create, response.audio.delta, etc.
- Audio format: PCM16 24kHz mono, base64 encoded

### xAI Realtime API
- Endpoint: `wss://api.x.ai/v1/realtime?model={model}`
- Similar event structure to OpenAI
- Check xAI docs for specific event names and audio format requirements
