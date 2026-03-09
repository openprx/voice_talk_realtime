# voice_talk_realtime

Browser-side real-time voice conversation library for OpenAI and xAI Realtime APIs. Pure Rust compiled to WebAssembly — no JavaScript framework required.

## Architecture

```
voice_talk_realtime/
├── src/                    # Core WASM library (browser)
│   ├── lib.rs              # Crate root, re-exports
│   ├── realtime/           # WebSocket clients
│   │   ├── protocol.rs     # Shared types: ServerEvent, ClientEvent, RealtimeClient trait
│   │   ├── openai.rs       # OpenAI Realtime API client
│   │   └── xai.rs          # xAI Realtime API client
│   ├── audio/              # Audio pipeline
│   │   ├── capture.rs      # Microphone capture via AudioWorklet
│   │   ├── playback.rs     # PCM16 playback via AudioContext
│   │   └── codec.rs        # PCM16 ↔ base64 conversion
│   └── web/                # JS-facing bindings
│       └── mod.rs          # VoiceTalkClient — unified WASM API
├── web-demo/               # Standalone HTML/JS/CSS demo
│   ├── index.html          # Single-file demo with PTT + VAD
│   ├── style.css           # Dark theme, responsive layout
│   └── js/                 # Audio worklet processors
├── prx-plugin/             # OpenPRX WASM plugin (signaling only)
│   ├── plugin.toml         # Plugin manifest
│   ├── Cargo.toml
│   └── src/lib.rs          # PDK exports: get_spec(), execute()
├── Cargo.toml
└── README.md
```

### Two WASM targets

| Component | Target | Purpose |
|-----------|--------|---------|
| `voice_talk_realtime` (root) | `wasm32-unknown-unknown` via wasm-pack | Browser library — WebSocket, audio capture/playback, UI bindings |
| `prx-plugin/` | `wasm32-unknown-unknown` via cargo | OpenPRX plugin — returns session config (WebSocket URL, auth, voice settings). No audio processing. |

## Supported providers

| Provider | Endpoint | Voices | Default model |
|----------|----------|--------|---------------|
| OpenAI | `wss://api.openai.com/v1/realtime` | alloy, ash, ballad, coral, echo, sage, shimmer, verse | `gpt-4o-realtime-preview-2024-12-17` |
| xAI | `wss://api.x.ai/v1/realtime` | Eve, Ara, Rex, Sal, Leo | `grok-3-fast-realtime` |

## Build

### Browser library (wasm-pack)

```bash
# Install wasm-pack if needed
curl https://rustwasm.github.io/wasm-pack/installer/init.sh -sSf | sh

# Build
wasm-pack build --target web --release

# Output: pkg/
#   voice_talk_realtime.js
#   voice_talk_realtime_bg.wasm
#   voice_talk_realtime.d.ts
```

### OpenPRX plugin

```bash
cd prx-plugin
cargo build --target wasm32-unknown-unknown --release

# Output: target/wasm32-unknown-unknown/release/voice_talk_realtime_plugin.wasm
```

### Development (native check)

```bash
# Quick type check (no wasm target needed)
cargo check

# Full WASM check
cargo check --target wasm32-unknown-unknown
```

## Usage

### Web demo

1. Build the WASM library (see above)
2. Serve `web-demo/` with any HTTP server:
   ```bash
   cd web-demo
   python3 -m http.server 8080
   ```
3. Open `http://localhost:8080`
4. Select provider, enter API key, choose voice
5. Click Connect, then use Push-to-Talk or enable VAD mode

### JavaScript integration

```javascript
import init, { VoiceTalkClient } from './pkg/voice_talk_realtime.js';

await init();

const client = new VoiceTalkClient('openai', apiKey, 'gpt-4o-realtime-preview-2024-12-17');
client.connect();

// Send text
client.send_text('Hello, how are you?');

// Send audio (base64-encoded PCM16)
client.append_audio_base64(base64AudioData);

// Commit audio buffer (triggers response)
client.commit_audio();

// Poll server events
const event = client.poll_event();

// Disconnect
client.disconnect();
```

### OpenPRX plugin

The plugin exposes a `voice_session` tool that returns connection configuration:

```json
{
  "provider": "openai",
  "voice": "alloy",
  "model": "gpt-4o-realtime-preview-2024-12-17",
  "instructions": "You are a helpful assistant.",
  "turn_detection": "server_vad"
}
```

Response:

```json
{
  "websocket_url": "wss://api.openai.com/v1/realtime?model=gpt-4o-realtime-preview-2024-12-17",
  "auth": { "type": "header", "name": "Authorization", "value": "Bearer <key>" },
  "session_config": { "modalities": ["text", "audio"], "voice": "alloy", ... }
}
```

## Features

- Push-to-Talk and VAD (Voice Activity Detection) modes
- Real-time audio visualization (playback + recording waveforms)
- Live transcript display
- Provider switching without reconnection
- Dark theme, responsive layout
- API key input with local-only warning

## Project stats

- ~1,230 lines of Rust (core library)
- ~330 lines of Rust (plugin)
- 15 commits on main

## License

Proprietary — ZeroClaw / OpenPRX
