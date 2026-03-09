use serde::{Deserialize, Serialize};

// ── Client → Server events ──────────────────────────────────────────

#[derive(Debug, Clone, Serialize)]
#[serde(tag = "type")]
pub enum ClientEvent {
    #[serde(rename = "session.update")]
    SessionUpdate { session: serde_json::Value },

    #[serde(rename = "input_audio_buffer.append")]
    InputAudioBufferAppend { audio: String },

    #[serde(rename = "input_audio_buffer.commit")]
    InputAudioBufferCommit {},

    #[serde(rename = "response.create")]
    ResponseCreate {
        #[serde(skip_serializing_if = "Option::is_none")]
        response: Option<serde_json::Value>,
    },
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

    #[serde(rename = "error")]
    Error {
        #[serde(default)]
        error: serde_json::Value,
    },

    /// Catch-all for events we don't explicitly handle yet.
    #[serde(other)]
    Unknown,
}
