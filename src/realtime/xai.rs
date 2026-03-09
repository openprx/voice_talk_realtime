use std::cell::RefCell;
use std::collections::VecDeque;
use std::rc::Rc;

use wasm_bindgen::closure::Closure;
use wasm_bindgen::JsCast;
use wasm_bindgen::JsValue;
use web_sys::{CloseEvent, ErrorEvent, Event, MessageEvent, WebSocket};

use crate::realtime::protocol::{ClientEvent, ServerEvent, RealtimeClient};

const XAI_ENDPOINT: &str = "wss://api.x.ai/v1/realtime";

/// Available xAI voices
pub enum XaiVoice {
    Tara,
    Sage,
    Ash,
    Coral,
    Ember,
}

impl XaiVoice {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Tara => "Tara",
            Self::Sage => "Sage",
            Self::Ash => "Ash",
            Self::Coral => "Coral",
            Self::Ember => "Ember",
        }
    }
}

struct ClientState {
    events: VecDeque<ServerEvent>,
    connected: bool,
}

/// xAI Realtime API authentication mode
pub enum XaiAuth {
    /// Server-side: pass API key as Bearer token (via subprotocol)
    ApiKey(String),
    /// Client-side: use ephemeral client secret token
    ClientSecret(String),
}

pub struct XaiRealtimeClient {
    auth: XaiAuth,
    model: String,
    ws: Option<WebSocket>,
    state: Rc<RefCell<ClientState>>,
    _on_open: Option<Closure<dyn FnMut(Event)>>,
    _on_message: Option<Closure<dyn FnMut(MessageEvent)>>,
    _on_error: Option<Closure<dyn FnMut(Event)>>,
    _on_close: Option<Closure<dyn FnMut(CloseEvent)>>,
}

impl XaiRealtimeClient {
    pub fn new() -> Self {
        Self::with_config(XaiAuth::ApiKey(String::new()), "grok-3-fast-realtime")
    }

    pub fn with_config(auth: XaiAuth, model: impl Into<String>) -> Self {
        Self {
            auth,
            model: model.into(),
            ws: None,
            state: Rc::new(RefCell::new(ClientState {
                events: VecDeque::new(),
                connected: false,
            })),
            _on_open: None,
            _on_message: None,
            _on_error: None,
            _on_close: None,
        }
    }

    pub fn set_auth(&mut self, auth: XaiAuth) {
        self.auth = auth;
    }

    pub fn set_model(&mut self, model: impl Into<String>) {
        self.model = model.into();
    }

    /// Build default session config for xAI
    pub fn default_session_config(&self, voice: &XaiVoice, instructions: &str) -> serde_json::Value {
        serde_json::json!({
            "voice": voice.as_str(),
            "instructions": instructions,
            "turn_detection": {
                "type": "server_vad"
            },
            "tools": [
                { "type": "web_search" },
                { "type": "x_search" }
            ],
            "input_audio_transcription": {
                "model": "grok-2-audio"
            },
            "audio": {
                "input": {
                    "format": { "type": "audio/pcm", "rate": 24000 }
                },
                "output": {
                    "format": { "type": "audio/pcm", "rate": 24000 }
                }
            }
        })
    }

    /// Convenience: send session.update
    pub fn send_session_update(&self, session: serde_json::Value) -> Result<(), JsValue> {
        self.send_event(&ClientEvent::SessionUpdate { session })
    }

    // Fix #1: send_text() — correct two-step: conversation.item.create + response.create
    /// Send a text message: create a conversation item then trigger a response.
    pub fn send_text(&self, text: &str) -> Result<(), JsValue> {
        let item = serde_json::json!({
            "type": "message",
            "role": "user",
            "content": [{
                "type": "input_text",
                "text": text
            }]
        });
        self.send_event(&ClientEvent::ConversationItemCreate { item })?;
        self.send_event(&ClientEvent::ResponseCreate { response: None })
    }

    // Fix #2: append_audio_base64 — only append, no auto-commit (server_vad handles it)
    /// Append base64-encoded audio. In server_vad mode, the server will
    /// automatically detect speech boundaries and trigger responses.
    pub fn append_audio_base64(&self, audio: impl Into<String>) -> Result<(), JsValue> {
        self.send_event(&ClientEvent::InputAudioBufferAppend {
            audio: audio.into(),
        })
    }

    /// Explicitly commit audio buffer (only needed in manual/push-to-talk mode)
    pub fn commit_audio(&self) -> Result<(), JsValue> {
        self.send_event(&ClientEvent::InputAudioBufferCommit {})
    }

    /// Clear audio buffer (useful on interruption)
    pub fn clear_audio(&self) -> Result<(), JsValue> {
        self.send_event(&ClientEvent::InputAudioBufferClear {})
    }

    /// Convenience: create response
    pub fn create_response(&self, response: Option<serde_json::Value>) -> Result<(), JsValue> {
        self.send_event(&ClientEvent::ResponseCreate { response })
    }

    /// Cancel an in-progress response (e.g. on user interruption)
    pub fn cancel_response(&self) -> Result<(), JsValue> {
        self.send_event(&ClientEvent::ResponseCancel {})
    }

    fn parse_server_event(raw: &str) -> Option<ServerEvent> {
        if let Ok(ev) = serde_json::from_str::<ServerEvent>(raw) {
            return Some(ev);
        }
        // xAI uses response.output_audio.delta instead of response.audio.delta
        let value: serde_json::Value = serde_json::from_str(raw).ok()?;
        let event_type = value.get("type")?.as_str()?;
        match event_type {
            "response.output_audio.delta" => Some(ServerEvent::ResponseAudioDelta {
                delta: value.get("delta").and_then(|v| v.as_str()).unwrap_or_default().to_string(),
            }),
            "response.output_audio_transcript.delta" => Some(ServerEvent::ResponseAudioTranscriptDelta {
                delta: value.get("delta").and_then(|v| v.as_str()).unwrap_or_default().to_string(),
            }),
            _ => Some(ServerEvent::Unknown),
        }
    }

    fn push_event(state: &Rc<RefCell<ClientState>>, event: ServerEvent) {
        state.borrow_mut().events.push_back(event);
    }
}

impl RealtimeClient for XaiRealtimeClient {
    fn connect(&mut self, url: &str) -> Result<(), JsValue> {
        let ws_url = if url.starts_with("wss://") || url.starts_with("ws://") {
            url.to_string()
        } else {
            XAI_ENDPOINT.to_string()
        };

        let protocols = js_sys::Array::new();
        match &self.auth {
            XaiAuth::ApiKey(key) => {
                if !key.is_empty() {
                    protocols.push(&JsValue::from_str(&format!("xai-insecure-api-key.{}", key)));
                }
            }
            XaiAuth::ClientSecret(token) => {
                protocols.push(&JsValue::from_str(&format!("xai-client-secret.{}", token)));
            }
        }

        let ws = if protocols.length() > 0 {
            WebSocket::new_with_str_sequence(&ws_url, &protocols)?
        } else {
            WebSocket::new(&ws_url)?
        };

        // onopen
        let open_state = Rc::clone(&self.state);
        let on_open = Closure::wrap(Box::new(move |_: Event| {
            open_state.borrow_mut().connected = true;
            Self::push_event(
                &open_state,
                ServerEvent::SessionCreated {
                    session: serde_json::Value::Null,
                },
            );
        }) as Box<dyn FnMut(Event)>);
        ws.set_onopen(Some(on_open.as_ref().unchecked_ref()));

        // onmessage
        let msg_state = Rc::clone(&self.state);
        let on_message = Closure::wrap(Box::new(move |event: MessageEvent| {
            if let Some(text) = event.data().as_string() {
                if let Some(parsed) = Self::parse_server_event(&text) {
                    Self::push_event(&msg_state, parsed);
                }
            }
        }) as Box<dyn FnMut(MessageEvent)>);
        ws.set_onmessage(Some(on_message.as_ref().unchecked_ref()));

        // onerror
        let err_state = Rc::clone(&self.state);
        let on_error = Closure::wrap(Box::new(move |event: Event| {
            let message = event
                .dyn_ref::<ErrorEvent>()
                .map(|e| e.message())
                .filter(|m| !m.is_empty())
                .unwrap_or_else(|| "websocket error".to_string());
            Self::push_event(
                &err_state,
                ServerEvent::Error {
                    error: serde_json::Value::String(message),
                },
            );
        }) as Box<dyn FnMut(Event)>);
        ws.set_onerror(Some(on_error.as_ref().unchecked_ref()));

        // onclose
        let close_state = Rc::clone(&self.state);
        let on_close = Closure::wrap(Box::new(move |event: CloseEvent| {
            close_state.borrow_mut().connected = false;
            let message = if event.reason().is_empty() {
                format!("websocket closed with code {}", event.code())
            } else {
                format!("websocket closed: {} ({})", event.reason(), event.code())
            };
            Self::push_event(
                &close_state,
                ServerEvent::Error {
                    error: serde_json::Value::String(message),
                },
            );
        }) as Box<dyn FnMut(CloseEvent)>);
        ws.set_onclose(Some(on_close.as_ref().unchecked_ref()));

        self._on_open = Some(on_open);
        self._on_message = Some(on_message);
        self._on_error = Some(on_error);
        self._on_close = Some(on_close);
        self.ws = Some(ws);
        Ok(())
    }

    fn send_event(&self, event: &ClientEvent) -> Result<(), JsValue> {
        let ws = self
            .ws
            .as_ref()
            .ok_or_else(|| JsValue::from_str("WebSocket not connected"))?;
        let payload = serde_json::to_string(event)
            .map_err(|e| JsValue::from_str(&format!("serialize error: {e}")))?;
        ws.send_with_str(&payload)
    }

    fn poll_event(&self) -> Option<ServerEvent> {
        self.state.borrow_mut().events.pop_front()
    }

    fn close(&self) {
        if let Some(ws) = &self.ws {
            let _ = ws.close();
        }
    }

    fn is_connected(&self) -> bool {
        self.state.borrow().connected
    }
}

impl Default for XaiRealtimeClient {
    fn default() -> Self {
        Self::new()
    }
}
