use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum ClientEvent {
    SessionUpdate {
        session: serde_json::Value,
    },
    InputAudioBufferAppend {
        audio: String,
    },
    InputAudioBufferCommit,
    ResponseCreate {
        response: serde_json::Value,
    },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum ServerEvent {
    SessionCreated {
        session: serde_json::Value,
    },
    ResponseTextDelta {
        delta: String,
    },
    ResponseAudioDelta {
        delta: String,
    },
    ResponseDone,
    Error {
        message: String,
    },
}
