pub mod openai;
pub mod protocol;

pub trait RealtimeClient {
    fn connect(&mut self, url: &str) -> Result<(), wasm_bindgen::JsValue>;
    fn send_text(&mut self, text: &str) -> Result<(), wasm_bindgen::JsValue>;
    fn send_audio(&mut self, audio_chunk: &[u8]) -> Result<(), wasm_bindgen::JsValue>;
    fn on_event(&mut self, callback: Box<dyn FnMut(protocol::ServerEvent)>);
    fn close(&mut self) -> Result<(), wasm_bindgen::JsValue>;
}
