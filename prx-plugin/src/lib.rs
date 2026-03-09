//! voice-talk-realtime OpenPRX Plugin
//!
//! Signaling-only plugin: returns WebSocket connection config for
//! real-time voice sessions. No audio processing — the browser
//! client handles WebSocket + audio directly.

use serde::{Deserialize, Serialize};
use serde_json::{json, Value};

// ── Plugin Spec ─────────────────────────────────────────────────────

#[derive(Serialize)]
struct ToolParam {
    name: &'static str,
    #[serde(rename = "type")]
    param_type: &'static str,
    required: bool,
    description: &'static str,
}

#[derive(Serialize)]
struct ToolSpec {
    name: &'static str,
    description: &'static str,
    parameters: Vec<ToolParam>,
}

#[derive(Serialize)]
struct PluginSpec {
    name: &'static str,
    version: &'static str,
    description: &'static str,
    capabilities: Vec<&'static str>,
    tools: Vec<ToolSpec>,
}

// ── Request / Response ──────────────────────────────────────────────

#[derive(Deserialize)]
struct VoiceSessionRequest {
    provider: String,
    #[serde(default)]
    voice: Option<String>,
    #[serde(default)]
    model: Option<String>,
    #[serde(default)]
    instructions: Option<String>,
    #[serde(default)]
    turn_detection: Option<String>,
}

// ── Constants ───────────────────────────────────────────────────────

const OPENAI_WS_URL: &str = "wss://api.openai.com/v1/realtime";
const XAI_WS_URL: &str = "wss://api.x.ai/v1/realtime";

const OPENAI_DEFAULT_MODEL: &str = "gpt-4o-realtime-preview-2024-12-17";
const XAI_DEFAULT_MODEL: &str = "grok-3-fast-realtime";

const OPENAI_DEFAULT_VOICE: &str = "verse";
const XAI_DEFAULT_VOICE: &str = "eve";

const OPENAI_VOICES: &[&str] = &["alloy", "ash", "ballad", "coral", "echo", "sage", "shimmer", "verse"];
const XAI_VOICES: &[&str] = &["eve", "ara", "rex", "sal", "leo"];

/// Length of the last returned string (for host-side ptr+len reads).
static mut LAST_RESULT_LEN: usize = 0;

// ── PDK Exports ─────────────────────────────────────────────────────

/// Plugin initialization. Called once when the plugin is loaded.
#[no_mangle]
pub extern "C" fn init() -> i32 {
    0 // success
}

/// Return plugin specification as JSON (allocated string).
/// Caller must free with `dealloc_str`.
#[no_mangle]
pub extern "C" fn describe() -> *mut u8 {
    let spec = PluginSpec {
        name: "voice-talk-realtime",
        version: "0.1.0",
        description: "Real-time voice conversation via OpenAI and xAI Realtime APIs",
        capabilities: vec!["audio", "realtime", "tools"],
        tools: vec![ToolSpec {
            name: "voice_session",
            description: "Create a real-time voice session configuration",
            parameters: vec![
                ToolParam { name: "provider", param_type: "string", required: true, description: "Provider: 'openai' or 'xai'" },
                ToolParam { name: "voice", param_type: "string", required: false, description: "Voice name" },
                ToolParam { name: "model", param_type: "string", required: false, description: "Model override" },
                ToolParam { name: "instructions", param_type: "string", required: false, description: "System instructions" },
                ToolParam { name: "turn_detection", param_type: "string", required: false, description: "'server_vad' or 'none'" },
            ],
        }],
    };

    let json = serde_json::to_string(&spec).unwrap_or_default();
    string_to_ptr(json)
}

/// Return tool specification for OpenPRX tool registry.
#[no_mangle]
pub extern "C" fn get_spec() -> *mut u8 {
    describe()
}

/// Execute a tool call. Input: JSON string pointer + length.
/// Returns JSON result as allocated string.
#[no_mangle]
pub extern "C" fn execute(ptr: *const u8, len: usize) -> *mut u8 {
    let input = unsafe {
        if ptr.is_null() || len == 0 {
            return string_to_ptr(error_json("empty input"));
        }
        let slice = std::slice::from_raw_parts(ptr, len);
        match std::str::from_utf8(slice) {
            Ok(s) => s.to_string(),
            Err(_) => return string_to_ptr(error_json("invalid UTF-8")),
        }
    };

    // Parse the wrapper: { "tool": "voice_session", "params": { ... } }
    let wrapper: Value = match serde_json::from_str(&input) {
        Ok(v) => v,
        Err(e) => return string_to_ptr(error_json(&format!("JSON parse error: {e}"))),
    };

    let tool_name = wrapper.get("tool").and_then(|v| v.as_str()).unwrap_or("");
    let params = wrapper.get("params").cloned().unwrap_or(Value::Null);

    match tool_name {
        "voice_session" => execute_voice_session(params),
        _ => string_to_ptr(error_json(&format!("unknown tool: {tool_name}"))),
    }
}

/// Handle events (e.g., audio data, connection state changes).
#[no_mangle]
pub extern "C" fn on_event(ptr: *const u8, len: usize) -> *mut u8 {
    let input = unsafe {
        if ptr.is_null() || len == 0 {
            return string_to_ptr(json!({"status": "ignored"}).to_string());
        }
        let slice = std::slice::from_raw_parts(ptr, len);
        std::str::from_utf8(slice).unwrap_or("").to_string()
    };

    let event: Value = serde_json::from_str(&input).unwrap_or(Value::Null);
    let event_type = event.get("type").and_then(|v| v.as_str()).unwrap_or("unknown");

    let response = match event_type {
        "session.connected" => json!({ "status": "ack", "message": "voice session connected" }),
        "session.disconnected" => json!({ "status": "ack", "message": "voice session ended" }),
        _ => json!({ "status": "ignored", "event_type": event_type }),
    };

    string_to_ptr(response.to_string())
}

/// Handle incoming messages (text from conversation).
#[no_mangle]
pub extern "C" fn on_message(ptr: *const u8, len: usize) -> *mut u8 {
    let input = unsafe {
        if ptr.is_null() || len == 0 {
            return string_to_ptr(json!({"action": "none"}).to_string());
        }
        let slice = std::slice::from_raw_parts(ptr, len);
        std::str::from_utf8(slice).unwrap_or("").to_string()
    };

    let msg: Value = serde_json::from_str(&input).unwrap_or(Value::Null);
    let text = msg.get("text").and_then(|v| v.as_str()).unwrap_or("");

    // Check if message is a voice session trigger
    let response = if text.starts_with("/voice") || text.starts_with("/talk") {
        json!({
            "action": "suggest_tool",
            "tool": "voice_session",
            "message": "Use voice_session tool to start a real-time voice conversation"
        })
    } else {
        json!({ "action": "none" })
    };

    string_to_ptr(response.to_string())
}

// ── Result length + Free allocated strings ──────────────────────────

/// Return the byte length of the last result string.
/// Host calls this after describe()/execute()/etc. to know how many bytes to read.
#[no_mangle]
pub extern "C" fn result_len() -> usize {
    unsafe { LAST_RESULT_LEN }
}

/// Free a string previously allocated by this plugin.
#[no_mangle]
pub extern "C" fn dealloc_str(ptr: *mut u8, len: usize) {
    if !ptr.is_null() && len > 0 {
        unsafe {
            let layout = std::alloc::Layout::from_size_align_unchecked(len, 1);
            std::alloc::dealloc(ptr, layout);
        }
    }
}

// ── Internal ────────────────────────────────────────────────────────

fn execute_voice_session(params: Value) -> *mut u8 {
    let req: VoiceSessionRequest = match serde_json::from_value(params) {
        Ok(r) => r,
        Err(e) => return string_to_ptr(error_json(&format!("invalid params: {e}"))),
    };

    let provider = req.provider.to_lowercase();

    let (ws_url, default_model, default_voice, valid_voices, auth_method) = match provider.as_str() {
        "openai" => (
            OPENAI_WS_URL,
            OPENAI_DEFAULT_MODEL,
            OPENAI_DEFAULT_VOICE,
            OPENAI_VOICES,
            "header",  // Authorization: Bearer <key>
        ),
        "xai" => (
            XAI_WS_URL,
            XAI_DEFAULT_MODEL,
            XAI_DEFAULT_VOICE,
            XAI_VOICES,
            "subprotocol",  // xai-client-secret.<token>
        ),
        _ => return string_to_ptr(error_json(&format!(
            "unsupported provider '{}'. Use 'openai' or 'xai'", provider
        ))),
    };

    let model = req.model.as_deref().unwrap_or(default_model);
    let voice = req.voice.as_deref().unwrap_or(default_voice);

    // Validate voice
    if !valid_voices.contains(&voice) {
        return string_to_ptr(error_json(&format!(
            "invalid voice '{}' for {}. Available: {:?}", voice, provider, valid_voices
        )));
    }

    let turn_detection = req.turn_detection.as_deref().unwrap_or("server_vad");
    let instructions = req.instructions.as_deref().unwrap_or("You are a helpful assistant.");

    // Build connection URL
    let connection_url = match provider.as_str() {
        "openai" => format!("{}?model={}", ws_url, model),
        "xai" => ws_url.to_string(),  // xAI doesn't use URL params
        _ => ws_url.to_string(),
    };

    // Build session config (sent as session.update after connection)
    let session_config = match provider.as_str() {
        "openai" => json!({
            "modalities": ["text", "audio"],
            "voice": voice,
            "instructions": instructions,
            "input_audio_format": "pcm16",
            "output_audio_format": "pcm16",
            "turn_detection": if turn_detection == "none" {
                Value::Null
            } else {
                json!({ "type": turn_detection })
            }
        }),
        "xai" => json!({
            "voice": voice,
            "instructions": instructions,
            "input_audio_format": "pcm16",
            "output_audio_format": "pcm16",
            "turn_detection": if turn_detection == "none" {
                Value::Null
            } else {
                json!({ "type": turn_detection })
            },
            "tools": [
                { "type": "web_search" },
                { "type": "x_search" }
            ],
            "input_audio_transcription": {
                "model": "grok-2-audio"
            }
        }),
        _ => json!({}),
    };

    let result = json!({
        "status": "ok",
        "provider": provider,
        "connection": {
            "url": connection_url,
            "auth_method": auth_method,
            "model": model,
        },
        "session_config": session_config,
        "voices": valid_voices,
        "audio_format": {
            "encoding": "pcm16",
            "sample_rate": 24000,
            "channels": 1,
        }
    });

    string_to_ptr(result.to_string())
}

fn error_json(msg: &str) -> String {
    json!({ "status": "error", "error": msg }).to_string()
}

fn string_to_ptr(s: String) -> *mut u8 {
    let mut bytes = s.into_bytes();
    bytes.shrink_to_fit();
    let len = bytes.len();
    let ptr = bytes.as_mut_ptr();
    std::mem::forget(bytes);
    unsafe { LAST_RESULT_LEN = len; }
    ptr
}
