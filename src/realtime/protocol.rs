use serde::{Deserialize, Serialize};
use serde_json::Value;

// ── Client → Server events ──────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum ClientEvent {
    #[serde(rename = "session.update")]
    SessionUpdate { session: serde_json::Value },

    #[serde(rename = "input_audio_buffer.append")]
    InputAudioBufferAppend { audio: String },

    #[serde(rename = "input_audio_buffer.commit")]
    InputAudioBufferCommit {},

    #[serde(rename = "input_audio_buffer.clear")]
    InputAudioBufferClear {},

    #[serde(rename = "response.create")]
    ResponseCreate {
        #[serde(skip_serializing_if = "Option::is_none")]
        response: Option<Value>,
    },

    // Fix #3: response.cancel
    #[serde(rename = "response.cancel")]
    ResponseCancel {},

    // Fix #5: conversation.item.create
    #[serde(rename = "conversation.item.create")]
    ConversationItemCreate { item: serde_json::Value },

    // Fix #5: conversation.item.delete
    #[serde(rename = "conversation.item.delete")]
    ConversationItemDelete { item_id: String },
}

// ── Server → Client events ──────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum ServerEvent {
    #[serde(rename = "session.created")]
    SessionCreated {
        #[serde(default)]
        session: serde_json::Value,
    },

    #[serde(rename = "session.updated")]
    SessionUpdated {
        #[serde(default)]
        session: serde_json::Value,
    },

    #[serde(rename = "response.text.delta")]
    ResponseTextDelta {
        #[serde(default)]
        delta: String,
    },

    #[serde(rename = "response.audio.delta")]
    ResponseAudioDelta {
        #[serde(default)]
        delta: String,
    },

    #[serde(rename = "response.audio_transcript.delta")]
    ResponseAudioTranscriptDelta {
        #[serde(default)]
        delta: String,
    },

    #[serde(rename = "response.done")]
    ResponseDone {
        #[serde(default)]
        response: serde_json::Value,
    },

    // Fix #4: input_audio_buffer.speech_started
    #[serde(rename = "input_audio_buffer.speech_started")]
    InputAudioBufferSpeechStarted {
        #[serde(default)]
        audio_start_ms: u64,
        #[serde(default)]
        item_id: String,
    },

    // Fix #4: input_audio_buffer.speech_stopped
    #[serde(rename = "input_audio_buffer.speech_stopped")]
    InputAudioBufferSpeechStopped {
        #[serde(default)]
        audio_end_ms: u64,
        #[serde(default)]
        item_id: String,
    },

    #[serde(rename = "input_audio_buffer.committed")]
    InputAudioBufferCommitted {
        #[serde(default)]
        item_id: String,
    },

    #[serde(rename = "response.created")]
    ResponseCreated {
        #[serde(default)]
        response: serde_json::Value,
    },

    #[serde(rename = "error")]
    Error {
        #[serde(default)]
        error: serde_json::Value,
    },

    /// Catch-all for events we don't explicitly handle yet.
    #[serde(other)]
    Unknown,
}

use wasm_bindgen::prelude::*;

pub type EventCallback = Box<dyn FnMut(ServerEvent)>;

/// Common trait for all realtime API providers
pub trait RealtimeClient {
    fn connect(&mut self, url: &str) -> Result<(), JsValue>;
    fn send_event(&self, event: &ClientEvent) -> Result<(), JsValue>;
    fn poll_event(&self) -> Option<ServerEvent>;
    fn close(&self);
    fn is_connected(&self) -> bool;
}
