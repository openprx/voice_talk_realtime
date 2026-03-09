use std::cell::RefCell;
use std::collections::VecDeque;
use std::rc::Rc;

use base64::Engine;
use serde_json::json;
use wasm_bindgen::closure::Closure;
use wasm_bindgen::JsCast;
use wasm_bindgen::JsValue;
use web_sys::{CloseEvent, ErrorEvent, Event, MessageEvent, WebSocket};

use crate::realtime::protocol::{ClientEvent, ServerEvent};
use crate::realtime::RealtimeClient;

type EventCallback = Box<dyn FnMut(ServerEvent)>;

struct ClientState {
    events: VecDeque<ServerEvent>,
    callback: Option<EventCallback>,
}

pub struct OpenAiRealtimeClient {
    api_key: String,
    model: String,
    ws: Option<WebSocket>,
    state: Rc<RefCell<ClientState>>,
    on_open: Option<Closure<dyn FnMut(Event)>>,
    on_message: Option<Closure<dyn FnMut(MessageEvent)>>,
    on_error: Option<Closure<dyn FnMut(Event)>>,
    on_close: Option<Closure<dyn FnMut(CloseEvent)>>,
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
                callback: None,
            })),
            on_open: None,
            on_message: None,
            on_error: None,
            on_close: None,
        }
    }

    pub fn set_api_key(&mut self, api_key: impl Into<String>) {
        self.api_key = api_key.into();
    }

    pub fn set_model(&mut self, model: impl Into<String>) {
        self.model = model.into();
    }

    pub fn send_session_update(&self, session: serde_json::Value) -> Result<(), JsValue> {
        self.send_client_event(&ClientEvent::SessionUpdate { session })
    }

    pub fn append_audio_base64(&self, audio: impl Into<String>) -> Result<(), JsValue> {
        self.send_client_event(&ClientEvent::InputAudioBufferAppend {
            audio: audio.into(),
        })
    }

    pub fn commit_audio(&self) -> Result<(), JsValue> {
        self.send_client_event(&ClientEvent::InputAudioBufferCommit)
    }

    pub fn create_response(&self, response: serde_json::Value) -> Result<(), JsValue> {
        self.send_client_event(&ClientEvent::ResponseCreate { response })
    }

    pub fn poll_event(&mut self) -> Option<ServerEvent> {
        self.state.borrow_mut().events.pop_front()
    }

    fn resolve_url(&self, url: &str) -> String {
        if url.starts_with("wss://") || url.starts_with("ws://") {
            return url.to_string();
        }

        let model = if !url.is_empty() { url } else { &self.model };
        format!("wss://api.openai.com/v1/realtime?model={model}")
    }

    fn send_client_event(&self, event: &ClientEvent) -> Result<(), JsValue> {
        let ws = self
            .ws
            .as_ref()
            .ok_or_else(|| JsValue::from_str("WebSocket is not connected"))?;
        let payload = serde_json::to_string(event)
            .map_err(|e| JsValue::from_str(&format!("failed to serialize client event: {e}")))?;
        ws.send_with_str(&payload)
    }

    fn push_event(state: &Rc<RefCell<ClientState>>, event: ServerEvent) {
        let mut state_mut = state.borrow_mut();
        state_mut.events.push_back(event.clone());
        if let Some(cb) = state_mut.callback.as_mut() {
            cb(event);
        }
    }

    fn parse_server_event(raw: &str) -> Option<ServerEvent> {
        let value: serde_json::Value = serde_json::from_str(raw).ok()?;
        let event_type = value.get("type")?.as_str()?;

        match event_type {
            "session.created" => Some(ServerEvent::SessionCreated {
                session: value
                    .get("session")
                    .cloned()
                    .unwrap_or(serde_json::Value::Null),
            }),
            "response.text.delta" => Some(ServerEvent::ResponseTextDelta {
                delta: value
                    .get("delta")
                    .and_then(|v| v.as_str())
                    .unwrap_or_default()
                    .to_string(),
            }),
            "response.audio.delta" | "response.output_audio.delta" => {
                Some(ServerEvent::ResponseAudioDelta {
                    delta: value
                        .get("delta")
                        .and_then(|v| v.as_str())
                        .unwrap_or_default()
                        .to_string(),
                })
            }
            "response.audio_transcript.delta" | "response.output_audio_transcript.delta" => {
                Some(ServerEvent::ResponseAudioTranscriptDelta {
                    delta: value
                        .get("delta")
                        .and_then(|v| v.as_str())
                        .unwrap_or_default()
                        .to_string(),
                })
            }
            "response.done" => Some(ServerEvent::ResponseDone),
            "error" => Some(ServerEvent::Error {
                message: value
                    .get("message")
                    .and_then(|v| v.as_str())
                    .unwrap_or("unknown websocket error")
                    .to_string(),
            }),
            _ => None,
        }
    }
}

impl RealtimeClient for OpenAiRealtimeClient {
    fn connect(&mut self, url: &str) -> Result<(), JsValue> {
        let ws_url = self.resolve_url(url);
        let protocols = js_sys::Array::new();
        protocols.push(&JsValue::from_str("realtime"));
        if !self.api_key.is_empty() {
            protocols.push(&JsValue::from_str(&format!(
                "openai-insecure-api-key.{}",
                self.api_key
            )));
        }
        protocols.push(&JsValue::from_str("openai-beta.realtime-v1"));

        let ws = WebSocket::new_with_str_sequence(&ws_url, &protocols)?;

        let open_state = Rc::clone(&self.state);
        let on_open = Closure::wrap(Box::new(move |_event: Event| {
            OpenAiRealtimeClient::push_event(
                &open_state,
                ServerEvent::SessionCreated {
                    session: serde_json::Value::Null,
                },
            );
        }) as Box<dyn FnMut(Event)>);
        ws.set_onopen(Some(on_open.as_ref().unchecked_ref()));

        let message_state = Rc::clone(&self.state);
        let on_message = Closure::wrap(Box::new(move |event: MessageEvent| {
            if let Some(text) = event.data().as_string() {
                if let Some(parsed) = OpenAiRealtimeClient::parse_server_event(&text) {
                    OpenAiRealtimeClient::push_event(&message_state, parsed);
                }
            }
        }) as Box<dyn FnMut(MessageEvent)>);
        ws.set_onmessage(Some(on_message.as_ref().unchecked_ref()));

        let error_state = Rc::clone(&self.state);
        let on_error = Closure::wrap(Box::new(move |event: Event| {
            let message = event
                .dyn_ref::<ErrorEvent>()
                .map(|e| e.message())
                .filter(|msg| !msg.is_empty())
                .unwrap_or_else(|| "websocket error".to_string());
            OpenAiRealtimeClient::push_event(&error_state, ServerEvent::Error { message });
        }) as Box<dyn FnMut(Event)>);
        ws.set_onerror(Some(on_error.as_ref().unchecked_ref()));

        let close_state = Rc::clone(&self.state);
        let on_close = Closure::wrap(Box::new(move |event: CloseEvent| {
            let message = if event.reason().is_empty() {
                format!("websocket closed with code {}", event.code())
            } else {
                format!(
                    "websocket closed with code {}: {}",
                    event.code(),
                    event.reason()
                )
            };
            OpenAiRealtimeClient::push_event(&close_state, ServerEvent::Error { message });
        }) as Box<dyn FnMut(CloseEvent)>);
        ws.set_onclose(Some(on_close.as_ref().unchecked_ref()));

        self.on_open = Some(on_open);
        self.on_message = Some(on_message);
        self.on_error = Some(on_error);
        self.on_close = Some(on_close);
        self.ws = Some(ws);
        Ok(())
    }

    fn send_text(&mut self, text: &str) -> Result<(), JsValue> {
        self.send_client_event(&ClientEvent::ResponseCreate {
            response: json!({
                "instructions": text
            }),
        })
    }

    fn send_audio(&mut self, audio_chunk: &[u8]) -> Result<(), JsValue> {
        let encoded = base64::engine::general_purpose::STANDARD.encode(audio_chunk);
        self.send_client_event(&ClientEvent::InputAudioBufferAppend { audio: encoded })?;
        self.send_client_event(&ClientEvent::InputAudioBufferCommit)?;
        self.send_client_event(&ClientEvent::ResponseCreate {
            response: serde_json::Value::Object(Default::default()),
        })
    }

    fn on_event(&mut self, callback: EventCallback) {
        self.state.borrow_mut().callback = Some(callback);
    }

    fn close(&mut self) -> Result<(), JsValue> {
        if let Some(ws) = &self.ws {
            ws.close()?;
        }
        self.ws = None;
        self.on_open = None;
        self.on_message = None;
        self.on_error = None;
        self.on_close = None;
        Ok(())
    }
}

impl Default for OpenAiRealtimeClient {
    fn default() -> Self {
        Self::new()
    }
}
