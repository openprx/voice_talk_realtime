use std::cell::RefCell;
use std::collections::VecDeque;
use std::rc::Rc;

use wasm_bindgen::closure::Closure;
use wasm_bindgen::JsCast;
use wasm_bindgen::JsValue;
use web_sys::{CloseEvent, ErrorEvent, Event, MessageEvent, WebSocket};

use crate::realtime::protocol::{ClientEvent, ServerEvent, RealtimeClient};

const MAX_EVENT_QUEUE: usize = 1000;

struct ClientState {
    events: VecDeque<ServerEvent>,
    connected: bool,
}

pub struct OpenAiRealtimeClient {
    api_key: String,
    model: String,
    ws: Option<WebSocket>,
    state: Rc<RefCell<ClientState>>,
    _on_open: Option<Closure<dyn FnMut(Event)>>,
    _on_message: Option<Closure<dyn FnMut(MessageEvent)>>,
    _on_error: Option<Closure<dyn FnMut(Event)>>,
    _on_close: Option<Closure<dyn FnMut(CloseEvent)>>,
}

impl OpenAiRealtimeClient {
    pub fn new() -> Self {
        Self::with_config("", "gpt-4o-realtime-preview-2024-12-17")
    }

    pub fn with_config(api_key: impl Into<String>, model: impl Into<String>) -> Self {
        Self {
            api_key: api_key.into(),
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

    pub fn set_api_key(&mut self, api_key: impl Into<String>) {
        self.api_key = api_key.into();
    }

    pub fn set_model(&mut self, model: impl Into<String>) {
        self.model = model.into();
    }

    /// Convenience: send session.update
    pub fn send_session_update(&self, session: serde_json::Value) -> Result<(), JsValue> {
        self.send_event(&ClientEvent::SessionUpdate { session })
    }

    /// Convenience: append base64-encoded audio
    pub fn append_audio_base64(&self, audio: impl Into<String>) -> Result<(), JsValue> {
        self.send_event(&ClientEvent::InputAudioBufferAppend {
            audio: audio.into(),
        })
    }

    /// Convenience: commit audio buffer
    pub fn commit_audio(&self) -> Result<(), JsValue> {
        self.send_event(&ClientEvent::InputAudioBufferCommit {})
    }

    /// Convenience: create response
    pub fn create_response(&self, response: Option<serde_json::Value>) -> Result<(), JsValue> {
        self.send_event(&ClientEvent::ResponseCreate { response })
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

    /// Clear audio buffer (useful on interruption)
    pub fn clear_audio(&self) -> Result<(), JsValue> {
        self.send_event(&ClientEvent::InputAudioBufferClear {})
    }

    /// Cancel an in-progress response (e.g. on user interruption)
    pub fn cancel_response(&self) -> Result<(), JsValue> {
        self.send_event(&ClientEvent::ResponseCancel {})
    }


    fn resolve_url(&self, url: &str) -> String {
        if url.starts_with("wss://") || url.starts_with("ws://") {
            return url.to_string();
        }
        let model = if !url.is_empty() { url } else { &self.model };
        format!("wss://api.openai.com/v1/realtime?model={model}")
    }

    fn parse_server_event(raw: &str) -> Option<ServerEvent> {
        let value: serde_json::Value = serde_json::from_str(raw).ok()?;
        let event_type = value.get("type")?.as_str()?;

        // Handle event name variants (e.g. OpenAI GA renames)
        match event_type {
            "response.output_audio.delta" => {
                return Some(ServerEvent::ResponseAudioDelta {
                    delta: value.get("delta").and_then(|v| v.as_str()).unwrap_or_default().to_string(),
                });
            }
            "response.output_audio_transcript.delta" => {
                return Some(ServerEvent::ResponseAudioTranscriptDelta {
                    delta: value.get("delta").and_then(|v| v.as_str()).unwrap_or_default().to_string(),
                });
            }
            _ => {}
        }

        // For all other events, use serde deserialization
        match serde_json::from_value::<ServerEvent>(value) {
            Ok(ev) => Some(ev),
            Err(_) => Some(ServerEvent::Unknown),
        }
    }

    fn push_event(state: &Rc<RefCell<ClientState>>, event: ServerEvent) {
        let mut s = state.borrow_mut();
        if s.events.len() > MAX_EVENT_QUEUE {
            s.events.pop_front();
        }
        s.events.push_back(event);
    }

    /// Clean up WebSocket and closures
    fn cleanup(&mut self) {
        if let Some(ws) = self.ws.take() {
            ws.set_onopen(None);
            ws.set_onmessage(None);
            ws.set_onerror(None);
            ws.set_onclose(None);
            let _ = ws.close();
        }
        self._on_open = None;
        self._on_message = None;
        self._on_error = None;
        self._on_close = None;
    }
}

impl RealtimeClient for OpenAiRealtimeClient {
    fn connect(&mut self, url: &str) -> Result<(), JsValue> {
        // H1: Clean up previous connection before reconnecting
        self.cleanup();
        self.state.borrow_mut().events.clear();

        let ws_url = self.resolve_url(url);
        let protocols = js_sys::Array::new();
        protocols.push(&JsValue::from_str("realtime"));
        if !self.api_key.is_empty() {
            // C1: API key in subprotocol — documented as browser-only workaround
            protocols.push(&JsValue::from_str(&format!(
                "openai-insecure-api-key.{}",
                self.api_key
            )));
        }
        protocols.push(&JsValue::from_str("openai-beta.realtime-v1"));

        let ws = WebSocket::new_with_str_sequence(&ws_url, &protocols)?;

        // H4: Only set connected flag on open, don't emit synthetic SessionCreated
        let open_state = Rc::clone(&self.state);
        let on_open = Closure::wrap(Box::new(move |_: Event| {
            open_state.borrow_mut().connected = true;
        }) as Box<dyn FnMut(Event)>);
        ws.set_onopen(Some(on_open.as_ref().unchecked_ref()));

        let msg_state = Rc::clone(&self.state);
        let on_message = Closure::wrap(Box::new(move |event: MessageEvent| {
            if let Some(text) = event.data().as_string() {
                if let Some(parsed) = OpenAiRealtimeClient::parse_server_event(&text) {
                    OpenAiRealtimeClient::push_event(&msg_state, parsed);
                }
            }
        }) as Box<dyn FnMut(MessageEvent)>);
        ws.set_onmessage(Some(on_message.as_ref().unchecked_ref()));

        let err_state = Rc::clone(&self.state);
        let on_error = Closure::wrap(Box::new(move |event: Event| {
            let message = event
                .dyn_ref::<ErrorEvent>()
                .map(|e| e.message())
                .filter(|m| !m.is_empty())
                .unwrap_or_else(|| "websocket error".to_string());
            OpenAiRealtimeClient::push_event(
                &err_state,
                ServerEvent::Error {
                    error: serde_json::Value::String(message),
                },
            );
        }) as Box<dyn FnMut(Event)>);
        ws.set_onerror(Some(on_error.as_ref().unchecked_ref()));

        let close_state = Rc::clone(&self.state);
        let on_close = Closure::wrap(Box::new(move |event: CloseEvent| {
            close_state.borrow_mut().connected = false;
            let message = if event.reason().is_empty() {
                format!("websocket closed with code {}", event.code())
            } else {
                format!("websocket closed: {} ({})", event.reason(), event.code())
            };
            OpenAiRealtimeClient::push_event(
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
        // M4: Check readyState before sending
        if ws.ready_state() != WebSocket::OPEN {
            return Err(JsValue::from_str("WebSocket not in OPEN state"));
        }
        let payload = serde_json::to_string(event)
            .map_err(|e| JsValue::from_str(&format!("serialize error: {e}")))?;
        ws.send_with_str(&payload)
    }

    fn poll_event(&self) -> Option<ServerEvent> {
        self.state.borrow_mut().events.pop_front()
    }

    // M6: close cleans up properly
    fn close(&self) {
        if let Some(ws) = &self.ws {
            ws.set_onopen(None);
            ws.set_onmessage(None);
            ws.set_onerror(None);
            ws.set_onclose(None);
            let _ = ws.close();
        }
    }

    fn is_connected(&self) -> bool {
        self.state.borrow().connected
    }
}

impl Default for OpenAiRealtimeClient {
    fn default() -> Self {
        Self::new()
    }
}
