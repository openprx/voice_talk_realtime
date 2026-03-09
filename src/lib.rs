use wasm_bindgen::prelude::*;

pub mod audio;
pub mod realtime;
pub mod web;

#[wasm_bindgen(start)]
pub fn start() -> Result<(), JsValue> {
    Ok(())
}
