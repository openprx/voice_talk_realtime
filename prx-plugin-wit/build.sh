#!/usr/bin/env bash
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
cd "$SCRIPT_DIR"

echo "==> Building voice-realtime WASM plugin..."

# Check prerequisites
if ! command -v cargo-component &>/dev/null; then
    echo "Error: cargo-component not found."
    echo "Install with: cargo install cargo-component"
    exit 1
fi

if ! rustup target list --installed | grep -q wasm32-wasip1; then
    echo "Adding wasm32-wasip1 target..."
    rustup target add wasm32-wasip1
fi

# Build
cargo component build --release

# Copy to plugin directory
WASM_FILE="target/wasm32-wasip1/release/voice_realtime_tool.wasm"
if [ -f "$WASM_FILE" ]; then
    cp "$WASM_FILE" plugin.wasm
    echo "==> Built: plugin.wasm ($(wc -c < plugin.wasm) bytes)"
else
    echo "Error: Build output not found at $WASM_FILE"
    exit 1
fi
