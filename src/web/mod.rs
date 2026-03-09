//! WASM bindings for the web demo.

use wasm_bindgen::prelude::*;

use crate::realtime::protocol::{ClientEvent, RealtimeClient};
use crate::realtime::openai::OpenAiRealtimeClient;
use crate::realtime::xai::{XaiRealtimeClient, XaiAuth};
use crate::audio::codec;

// ── Shared client wrapper ───────────────────────────────────────────

enum ClientInner {
    OpenAi(OpenAiRealtimeClient),
    Xai(XaiRealtimeClient),
}

/// Opaque handle to a realtime client, exported to JS.
#[wasm_bindgen]
pub struct VoiceTalkClient {
    inner: ClientInner,
}

#[wasm_bindgen]
impl VoiceTalkClient {
    /// Create a new client for the given provider ("openai" or "xai").
    #[wasm_bindgen(constructor)]
    pub fn new(provider: &str, api_key: &str, model: &str) -> Self {
        let inner = match provider {
            "xai" => ClientInner::Xai(XaiRealtimeClient::with_config(
                XaiAuth::ClientSecret(api_key.to_string()),
                model,
            )),
            _ => ClientInner::OpenAi(OpenAiRealtimeClient::with_config(api_key, model)),
        };
        Self { inner }
    }

    /// Connect to the realtime API.
    /// `url` can be empty to use the default endpoint for the provider.
    pub fn connect(&mut self, url: &str) -> Result<(), JsValue> {
        match &mut self.inner {
            ClientInner::OpenAi(c) => c.connect(url),
            ClientInner::Xai(c) => c.connect(url),
        }
    }

    /// Send a client event as a JSON string.
    pub fn send_event_json(&self, json_str: &str) -> Result<(), JsValue> {
        let event: ClientEvent = serde_json::from_str(json_str)
            .map_err(|e| JsValue::from_str(&format!("invalid event JSON: {e}")))?;
        match &self.inner {
            ClientInner::OpenAi(c) => c.send_event(&event),
            ClientInner::Xai(c) => c.send_event(&event),
        }
    }

    /// Send a session.update event.
    pub fn send_session_update(&self, session_json: &str) -> Result<(), JsValue> {
        let session: serde_json::Value = serde_json::from_str(session_json)
            .map_err(|e| JsValue::from_str(&format!("invalid session JSON: {e}")))?;
        let event = ClientEvent::SessionUpdate { session };
        match &self.inner {
            ClientInner::OpenAi(c) => c.send_event(&event),
            ClientInner::Xai(c) => c.send_event(&event),
        }
    }

    /// Append base64-encoded audio to the input buffer.
    pub fn append_audio(&self, audio_b64: &str) -> Result<(), JsValue> {
        let event = ClientEvent::InputAudioBufferAppend { audio: audio_b64.to_string() };
        match &self.inner {
            ClientInner::OpenAi(c) => c.send_event(&event),
            ClientInner::Xai(c) => c.send_event(&event),
        }
    }

    /// Commit the input audio buffer.
    pub fn commit_audio(&self) -> Result<(), JsValue> {
        let event = ClientEvent::InputAudioBufferCommit {};
        match &self.inner {
            ClientInner::OpenAi(c) => c.send_event(&event),
            ClientInner::Xai(c) => c.send_event(&event),
        }
    }

    /// Request a response.
    pub fn create_response(&self) -> Result<(), JsValue> {
        let event = ClientEvent::ResponseCreate { response: None };
        match &self.inner {
            ClientInner::OpenAi(c) => c.send_event(&event),
            ClientInner::Xai(c) => c.send_event(&event),
        }
    }

    /// Poll the next server event as a JSON string, or null if none.
    pub fn poll_event(&self) -> Option<String> {
        let event = match &self.inner {
            ClientInner::OpenAi(c) => c.poll_event(),
            ClientInner::Xai(c) => c.poll_event(),
        };
        event.and_then(|e| serde_json::to_string(&e).ok())
    }

    /// Check if connected.
    pub fn is_connected(&self) -> bool {
        match &self.inner {
            ClientInner::OpenAi(c) => c.is_connected(),
            ClientInner::Xai(c) => c.is_connected(),
        }
    }

    /// Close the connection.
    pub fn close(&self) {
        match &self.inner {
            ClientInner::OpenAi(c) => c.close(),
            ClientInner::Xai(c) => c.close(),
        }
    }
}

// ── Standalone codec helpers exported to JS ─────────────────────────

/// Encode PCM16 LE bytes (Uint8Array) to base64 string.
#[wasm_bindgen]
pub fn pcm16_to_base64(pcm_data: &[u8]) -> String {
    codec::pcm16_to_base64(pcm_data)
}

/// Decode base64 string to PCM16 LE bytes (Uint8Array).
#[wasm_bindgen]
pub fn base64_to_pcm16(encoded: &str) -> Result<Vec<u8>, JsValue> {
    codec::base64_to_pcm16(encoded)
        .map_err(|e| JsValue::from_str(&format!("{e}")))
}

/// Convert f32 samples to PCM16 LE bytes.
#[wasm_bindgen]
pub fn f32_to_pcm16(samples: &[f32]) -> Vec<u8> {
    codec::f32_to_pcm16(samples)
}

/// Convert PCM16 LE bytes to f32 samples.
#[wasm_bindgen]
pub fn pcm16_to_f32(pcm_data: &[u8]) -> Vec<f32> {
    codec::pcm16_to_f32(pcm_data)
}
