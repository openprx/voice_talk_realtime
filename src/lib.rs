use wasm_bindgen::prelude::*;

pub mod audio;
pub mod realtime;
pub mod web;

// Re-export key types at crate root for convenience
pub use realtime::{OpenAiRealtimeClient, XaiRealtimeClient, RealtimeClient};
pub use web::VoiceTalkClient;

#[wasm_bindgen(start)]
pub fn start() -> Result<(), JsValue> {
    Ok(())
}
