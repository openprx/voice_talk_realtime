use std::cell::RefCell;
use std::collections::VecDeque;
use std::rc::Rc;

use wasm_bindgen::prelude::*;
use wasm_bindgen::JsCast;
use web_sys::{MessageEvent, WebSocket};

use super::protocol::{ClientEvent, ServerEvent};

/// Shared interior state behind Rc<RefCell<…>> so closures can mutate it.
struct Inner {
    ws: Option<WebSocket>,
    events: VecDeque<ServerEvent>,
    is_open: bool,
    /// Keep closures alive for the lifetime of the connection.
    _on_open: Option<Closure<dyn FnMut(JsValue)>>,
    _on_message: Option<Closure<dyn FnMut(MessageEvent)>>,
    _on_error: Option<Closure<dyn FnMut(JsValue)>>,
    _on_close: Option<Closure<dyn FnMut(JsValue)>>,
}

/// A WebSocket-based client for the OpenAI Realtime API, designed to run
/// inside a WASM environment (browser).
pub struct OpenAiRealtimeClient {
    inner: Rc<RefCell<Inner>>,
}

impl OpenAiRealtimeClient {
    pub fn new() -> Self {
        Self {
            inner: Rc::new(RefCell::new(Inner {
                ws: None,
                events: VecDeque::new(),
                is_open: false,
                _on_open: None,
                _on_message: None,
                _on_error: None,
                _on_close: None,
            })),
        }
    }

    /// Connect to the OpenAI Realtime API.
    ///
    /// `model` – e.g. `"gpt-4o-realtime-preview-2024-12-17"`
    /// `api_key` – your OpenAI API key (sent via the sub-protocol header trick).
    ///
    /// The browser WebSocket API does not allow arbitrary headers, so we pass
    /// the API key and model info via the `Sec-WebSocket-Protocol` header by
    /// supplying them as sub-protocols:
    ///   protocols = ["realtime", "openai-insecure-api-key.<KEY>", "openai-beta.realtime-v1"]
    pub fn connect(&self, model: &str, api_key: &str) -> Result<(), JsValue> {
        let url = format!(
            "wss://api.openai.com/v1/realtime?model={}",
            js_sys::encode_uri_component(model)
        );

        // Build the protocols array
        let protocols = js_sys::Array::new();
        protocols.push(&JsValue::from_str("realtime"));
        protocols.push(&JsValue::from_str(&format!(
            "openai-insecure-api-key.{}",
            api_key
        )));
        protocols.push(&JsValue::from_str("openai-beta.realtime-v1"));

        let ws = WebSocket::new_with_str_sequence(&url, &protocols)?;
        ws.set_binary_type(web_sys::BinaryType::Arraybuffer);

        // ── onopen ──────────────────────────────────────────────
        let inner_ref = Rc::clone(&self.inner);
        let on_open = Closure::<dyn FnMut(JsValue)>::new(move |_event: JsValue| {
            if let Ok(mut inner) = inner_ref.try_borrow_mut() {
                inner.is_open = true;
                web_sys::console::log_1(&"[voice_talk] WebSocket connected".into());
            }
        });
        ws.set_onopen(Some(on_open.as_ref().unchecked_ref()));

        // ── onmessage ───────────────────────────────────────────
        let inner_ref = Rc::clone(&self.inner);
        let on_message = Closure::<dyn FnMut(MessageEvent)>::new(move |ev: MessageEvent| {
            if let Some(text) = ev.data().as_string() {
                match serde_json::from_str::<ServerEvent>(&text) {
                    Ok(event) => {
                        if let Ok(mut inner) = inner_ref.try_borrow_mut() {
                            inner.events.push_back(event);
                        }
                    }
                    Err(e) => {
                        web_sys::console::warn_1(
                            &format!("[voice_talk] Failed to parse server event: {}", e).into(),
                        );
                    }
                }
            }
        });
        ws.set_onmessage(Some(on_message.as_ref().unchecked_ref()));

        // ── onerror ─────────────────────────────────────────────
        let inner_ref = Rc::clone(&self.inner);
        let on_error = Closure::<dyn FnMut(JsValue)>::new(move |err: JsValue| {
            web_sys::console::error_1(&format!("[voice_talk] WebSocket error: {:?}", err).into());
            if let Ok(mut inner) = inner_ref.try_borrow_mut() {
                inner.is_open = false;
            }
        });
        ws.set_onerror(Some(on_error.as_ref().unchecked_ref()));

        // ── onclose ─────────────────────────────────────────────
        let inner_ref = Rc::clone(&self.inner);
        let on_close = Closure::<dyn FnMut(JsValue)>::new(move |_ev: JsValue| {
            web_sys::console::log_1(&"[voice_talk] WebSocket closed".into());
            if let Ok(mut inner) = inner_ref.try_borrow_mut() {
                inner.is_open = false;
            }
        });
        ws.set_onclose(Some(on_close.as_ref().unchecked_ref()));

        // Store everything
        {
            let mut inner = self.inner.borrow_mut();
            inner.ws = Some(ws);
            inner._on_open = Some(on_open);
            inner._on_message = Some(on_message);
            inner._on_error = Some(on_error);
            inner._on_close = Some(on_close);
        }

        Ok(())
    }

    /// Send a client event (JSON) over the WebSocket.
    pub fn send_event(&self, event: &ClientEvent) -> Result<(), JsValue> {
        let inner = self.inner.borrow();
        let ws = inner
            .ws
            .as_ref()
            .ok_or_else(|| JsValue::from_str("WebSocket not connected"))?;
        if !inner.is_open {
            return Err(JsValue::from_str("WebSocket not open"));
        }
        let json = serde_json::to_string(event)
            .map_err(|e| JsValue::from_str(&format!("serialize error: {}", e)))?;
        ws.send_with_str(&json)
    }

    /// Convenience: send a `session.update` event.
    pub fn send_session_update(&self, session: serde_json::Value) -> Result<(), JsValue> {
        self.send_event(&ClientEvent::SessionUpdate { session })
    }

    /// Convenience: append base64-encoded audio to the input buffer.
    pub fn send_audio(&self, base64_audio: &str) -> Result<(), JsValue> {
        self.send_event(&ClientEvent::InputAudioBufferAppend {
            audio: base64_audio.to_string(),
        })
    }

    /// Convenience: commit the input audio buffer.
    pub fn commit_audio(&self) -> Result<(), JsValue> {
        self.send_event(&ClientEvent::InputAudioBufferCommit {})
    }

    /// Convenience: request the model to generate a response.
    pub fn create_response(&self, response: Option<serde_json::Value>) -> Result<(), JsValue> {
        self.send_event(&ClientEvent::ResponseCreate { response })
    }

    /// Poll the next received server event (non-blocking).
    pub fn poll_event(&self) -> Option<ServerEvent> {
        self.inner.borrow_mut().events.pop_front()
    }

    /// Drain all queued server events.
    pub fn drain_events(&self) -> Vec<ServerEvent> {
        self.inner.borrow_mut().events.drain(..).collect()
    }

    /// Whether the WebSocket is currently open.
    pub fn is_connected(&self) -> bool {
        self.inner.borrow().is_open
    }

    /// Close the WebSocket connection.
    pub fn close(&self) -> Result<(), JsValue> {
        let inner = self.inner.borrow();
        if let Some(ws) = &inner.ws {
            ws.close()
        } else {
            Ok(())
        }
    }
}

impl Default for OpenAiRealtimeClient {
    fn default() -> Self {
        Self::new()
    }
}
