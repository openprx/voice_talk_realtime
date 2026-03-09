use crate::realtime::{protocol::ServerEvent, RealtimeClient};

pub struct OpenAiRealtimeClient;

impl OpenAiRealtimeClient {
    pub fn new() -> Self {
        Self
    }
}

impl RealtimeClient for OpenAiRealtimeClient {
    fn connect(&mut self, _url: &str) -> Result<(), wasm_bindgen::JsValue> {
        Ok(())
    }

    fn send_text(&mut self, _text: &str) -> Result<(), wasm_bindgen::JsValue> {
        Ok(())
    }

    fn send_audio(&mut self, _audio_chunk: &[u8]) -> Result<(), wasm_bindgen::JsValue> {
        Ok(())
    }

    fn on_event(&mut self, _callback: Box<dyn FnMut(ServerEvent)>) {}

    fn close(&mut self) -> Result<(), wasm_bindgen::JsValue> {
        Ok(())
    }
}
