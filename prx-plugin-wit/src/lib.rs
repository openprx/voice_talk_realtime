//! # voice-realtime-tool
//!
//! PRX WASM tool plugin: real-time voice session configuration.
//!
//! Signaling-only — returns WebSocket connection config for
//! OpenAI and xAI Realtime APIs.

use prx_pdk::prelude::*;

#[cfg(target_arch = "wasm32")]
mod bindings;

// ── Constants ───────────────────────────────────────────────────────

const OPENAI_WS_URL: &str = "wss://api.openai.com/v1/realtime";
const XAI_WS_URL: &str = "wss://api.x.ai/v1/realtime";

const OPENAI_DEFAULT_MODEL: &str = "gpt-4o-realtime-preview-2024-12-17";
const XAI_DEFAULT_MODEL: &str = "grok-3-fast-realtime";

const OPENAI_DEFAULT_VOICE: &str = "verse";
const XAI_DEFAULT_VOICE: &str = "eve";

const OPENAI_VOICES: &[&str] = &["alloy", "ash", "ballad", "coral", "echo", "sage", "shimmer", "verse"];
const XAI_VOICES: &[&str] = &["eve", "ara", "rex", "sal", "leo"];

// ── Request type ────────────────────────────────────────────────────

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

// ── Plugin implementation ───────────────────────────────────────────

pub struct VoiceRealtimeTool;

impl VoiceRealtimeTool {
    pub fn get_spec_impl() -> ToolSpec {
        ToolSpec {
            name: "voice_session".to_string(),
            description: "Create a real-time voice session configuration. Returns WebSocket \
                          URL, auth method, session config, and available voices for the \
                          specified provider (OpenAI or xAI)."
                .to_string(),
            parameters_schema: r#"{
  "type": "object",
  "properties": {
    "provider": {
      "type": "string",
      "enum": ["openai", "xai"],
      "description": "Provider: 'openai' or 'xai'"
    },
    "voice": {
      "type": "string",
      "description": "Voice name. OpenAI: alloy/ash/ballad/coral/echo/sage/shimmer/verse. xAI: eve/ara/rex/sal/leo"
    },
    "model": {
      "type": "string",
      "description": "Model override"
    },
    "instructions": {
      "type": "string",
      "description": "System instructions for the voice session"
    },
    "turn_detection": {
      "type": "string",
      "enum": ["server_vad", "none"],
      "description": "Turn detection mode: 'server_vad' (auto) or 'none' (push-to-talk)"
    }
  },
  "required": ["provider"]
}"#
            .to_string(),
        }
    }

    pub fn execute_impl(args_json: &str) -> PluginResult {
        let req: VoiceSessionRequest = match serde_json::from_str(args_json) {
            Ok(r) => r,
            Err(e) => return PluginResult::err(format!("Invalid args: {e}")),
        };

        let provider = req.provider.to_lowercase();

        let (ws_url, default_model, default_voice, valid_voices, auth_method) = match provider.as_str() {
            "openai" => (OPENAI_WS_URL, OPENAI_DEFAULT_MODEL, OPENAI_DEFAULT_VOICE, OPENAI_VOICES, "header"),
            "xai" => (XAI_WS_URL, XAI_DEFAULT_MODEL, XAI_DEFAULT_VOICE, XAI_VOICES, "subprotocol"),
            _ => return PluginResult::err(format!("Unsupported provider '{}'. Use 'openai' or 'xai'", provider)),
        };

        let model = req.model.as_deref().unwrap_or(default_model);
        let voice = req.voice.as_deref().unwrap_or(default_voice);

        if !valid_voices.contains(&voice) {
            return PluginResult::err(format!(
                "Invalid voice '{}' for {}. Available: {:?}", voice, provider, valid_voices
            ));
        }

        let turn_detection = req.turn_detection.as_deref().unwrap_or("server_vad");
        let instructions = req.instructions.as_deref().unwrap_or("You are a helpful assistant.");

        let connection_url = match provider.as_str() {
            "openai" => format!("{}?model={}", ws_url, model),
            _ => ws_url.to_string(),
        };

        let td_value = if turn_detection == "none" {
            serde_json::Value::Null
        } else {
            json!({ "type": turn_detection })
        };

        let session_config = match provider.as_str() {
            "openai" => json!({
                "modalities": ["text", "audio"],
                "voice": voice,
                "instructions": instructions,
                "input_audio_format": "pcm16",
                "output_audio_format": "pcm16",
                "turn_detection": td_value
            }),
            "xai" => json!({
                "voice": voice,
                "instructions": instructions,
                "input_audio_format": "pcm16",
                "output_audio_format": "pcm16",
                "turn_detection": td_value,
                "tools": [{"type": "web_search"}, {"type": "x_search"}],
                "input_audio_transcription": {"model": "grok-2-audio"}
            }),
            _ => json!({}),
        };

        let _ = kv::increment(&format!("{}_sessions", provider), 1);

        log::info(&format!("voice_session: provider={} model={} voice={} td={}", provider, model, voice, turn_detection));

        let result = json!({
            "status": "ok",
            "provider": provider,
            "connection": {"url": connection_url, "auth_method": auth_method, "model": model},
            "session_config": session_config,
            "voices": valid_voices,
            "audio_format": {"encoding": "pcm16", "sample_rate": 24000, "channels": 1}
        });

        PluginResult::ok(result.to_string())
    }
}

// ── WIT guest trait (cargo-component generates bindings module) ─────

#[cfg(target_arch = "wasm32")]
mod wasm_exports {
    use super::VoiceRealtimeTool;
    use crate::bindings::exports::prx::plugin::tool_exports as wit;

    impl wit::Guest for VoiceRealtimeTool {
        fn get_spec() -> wit::ToolSpec {
            let s = VoiceRealtimeTool::get_spec_impl();
            wit::ToolSpec {
                name: s.name,
                description: s.description,
                parameters_schema: s.parameters_schema,
            }
        }

        fn execute(args: String) -> wit::PluginResult {
            let r = VoiceRealtimeTool::execute_impl(&args);
            wit::PluginResult {
                success: r.success,
                output: r.output,
                error: r.error,
            }
        }
    }

    crate::bindings::export!(VoiceRealtimeTool with_types_in crate::bindings);
}

// ── Tests ───────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn spec_has_required_fields() {
        let spec = VoiceRealtimeTool::get_spec_impl();
        assert_eq!(spec.name, "voice_session");
        assert!(!spec.description.is_empty());
        let schema: serde_json::Value = serde_json::from_str(&spec.parameters_schema).unwrap();
        assert!(schema["properties"]["provider"].is_object());
        assert_eq!(schema["required"][0], "provider");
    }

    #[test]
    fn xai_default_config() {
        let r = VoiceRealtimeTool::execute_impl(r#"{"provider":"xai"}"#);
        assert!(r.success, "error: {:?}", r.error);
        let v: serde_json::Value = serde_json::from_str(&r.output).unwrap();
        assert_eq!(v["provider"], "xai");
        assert_eq!(v["connection"]["url"], "wss://api.x.ai/v1/realtime");
        assert_eq!(v["connection"]["auth_method"], "subprotocol");
        assert_eq!(v["connection"]["model"], "grok-3-fast-realtime");
        assert_eq!(v["session_config"]["voice"], "eve");
    }

    #[test]
    fn openai_default_config() {
        let r = VoiceRealtimeTool::execute_impl(r#"{"provider":"openai"}"#);
        assert!(r.success, "error: {:?}", r.error);
        let v: serde_json::Value = serde_json::from_str(&r.output).unwrap();
        assert_eq!(v["provider"], "openai");
        assert!(v["connection"]["url"].as_str().unwrap().starts_with("wss://api.openai.com"));
        assert_eq!(v["connection"]["auth_method"], "header");
    }

    #[test]
    fn custom_voice_xai() {
        let r = VoiceRealtimeTool::execute_impl(r#"{"provider":"xai","voice":"rex"}"#);
        assert!(r.success);
        let v: serde_json::Value = serde_json::from_str(&r.output).unwrap();
        assert_eq!(v["session_config"]["voice"], "rex");
    }

    #[test]
    fn invalid_voice_rejected() {
        let r = VoiceRealtimeTool::execute_impl(r#"{"provider":"xai","voice":"invalid"}"#);
        assert!(!r.success);
        assert!(r.error.as_deref().unwrap().contains("Invalid voice"));
    }

    #[test]
    fn invalid_provider_rejected() {
        let r = VoiceRealtimeTool::execute_impl(r#"{"provider":"google"}"#);
        assert!(!r.success);
        assert!(r.error.as_deref().unwrap().contains("Unsupported provider"));
    }

    #[test]
    fn ptt_mode() {
        let r = VoiceRealtimeTool::execute_impl(r#"{"provider":"openai","turn_detection":"none"}"#);
        assert!(r.success);
        let v: serde_json::Value = serde_json::from_str(&r.output).unwrap();
        assert!(v["session_config"]["turn_detection"].is_null());
    }

    #[test]
    fn custom_instructions() {
        let r = VoiceRealtimeTool::execute_impl(r#"{"provider":"xai","instructions":"You are a pirate."}"#);
        assert!(r.success);
        let v: serde_json::Value = serde_json::from_str(&r.output).unwrap();
        assert_eq!(v["session_config"]["instructions"], "You are a pirate.");
    }

    #[test]
    fn xai_has_tools_in_config() {
        let r = VoiceRealtimeTool::execute_impl(r#"{"provider":"xai"}"#);
        assert!(r.success);
        let v: serde_json::Value = serde_json::from_str(&r.output).unwrap();
        assert!(v["session_config"]["tools"].is_array());
    }

    #[test]
    fn audio_format_correct() {
        let r = VoiceRealtimeTool::execute_impl(r#"{"provider":"openai"}"#);
        assert!(r.success);
        let v: serde_json::Value = serde_json::from_str(&r.output).unwrap();
        assert_eq!(v["audio_format"]["encoding"], "pcm16");
        assert_eq!(v["audio_format"]["sample_rate"], 24000);
        assert_eq!(v["audio_format"]["channels"], 1);
    }
}
