//! Microphone capture via Web Audio API + AudioWorklet.
//!
//! Flow: getUserMedia → MediaStreamSource → AudioWorkletNode → PCM chunks → callback
//!
//! The actual AudioWorklet processor runs in JS (web-demo/js/audio_worklet.js).
//! This module provides the Rust/WASM glue to set up the pipeline and receive chunks.

use wasm_bindgen::prelude::*;
use wasm_bindgen::JsCast;
use web_sys::{AudioContext, AudioWorkletNode, MediaStream, MediaStreamConstraints};

/// Microphone capture manager using AudioWorklet.
pub struct AudioCapture {
    ctx: Option<AudioContext>,
    worklet_node: Option<AudioWorkletNode>,
    _stream: Option<MediaStream>,
    is_capturing: bool,
}

impl AudioCapture {
    pub fn new() -> Self {
        Self {
            ctx: None,
            worklet_node: None,
            _stream: None,
            is_capturing: false,
        }
    }

    /// Initialize the AudioContext and register the AudioWorklet processor.
    /// `worklet_url` is the URL to the audio_worklet.js file.
    pub async fn init(&mut self, worklet_url: &str) -> Result<(), JsValue> {
        let ctx = AudioContext::new()?;

        let worklet = ctx.audio_worklet()?;
        let promise = worklet.add_module(worklet_url)?;
        wasm_bindgen_futures::JsFuture::from(promise).await?;

        self.ctx = Some(ctx);
        Ok(())
    }

    /// Start capturing audio from the user's microphone.
    /// `on_pcm_chunk` is a JS callback called with base64-encoded PCM16 24kHz chunks.
    pub async fn start(&mut self, on_pcm_chunk: js_sys::Function) -> Result<(), JsValue> {
        let ctx = self
            .ctx
            .as_ref()
            .ok_or_else(|| JsValue::from_str("AudioContext not initialized; call init() first"))?;

        let _ = wasm_bindgen_futures::JsFuture::from(ctx.resume()?).await;

        let window = web_sys::window().ok_or_else(|| JsValue::from_str("no window"))?;
        let navigator = window.navigator();
        let media_devices = navigator
            .media_devices()
            .map_err(|_| JsValue::from_str("no media devices"))?;

        let constraints = MediaStreamConstraints::new();
        constraints.set_audio(&JsValue::TRUE);
        constraints.set_video(&JsValue::FALSE);

        let promise = media_devices.get_user_media_with_constraints(&constraints)?;
        let stream_js: JsValue = wasm_bindgen_futures::JsFuture::from(promise).await?;
        let stream: MediaStream = stream_js.unchecked_into();

        let source = ctx.create_media_stream_source(&stream)?;

        let worklet_node = AudioWorkletNode::new(ctx, "pcm-capture-processor")?;

        let port = worklet_node.port()?;
        let callback = Closure::wrap(Box::new(move |event: web_sys::MessageEvent| {
            let _ = on_pcm_chunk.call1(&JsValue::NULL, &event.data());
        }) as Box<dyn FnMut(web_sys::MessageEvent)>);
        port.set_onmessage(Some(callback.as_ref().unchecked_ref()));
        callback.forget();

        source.connect_with_audio_node(&worklet_node)?;

        self._stream = Some(stream);
        self.worklet_node = Some(worklet_node);
        self.is_capturing = true;
        Ok(())
    }

    /// Stop capturing audio.
    pub fn stop(&mut self) -> Result<(), JsValue> {
        if let Some(node) = self.worklet_node.take() {
            node.disconnect()?;
        }
        if let Some(stream) = self._stream.take() {
            let tracks = stream.get_tracks();
            for i in 0..tracks.length() {
                let track_val = tracks.get(i);
                let track: web_sys::MediaStreamTrack = track_val.unchecked_into();
                track.stop();
            }
        }
        self.is_capturing = false;
        Ok(())
    }

    /// Whether capture is currently active.
    pub fn is_capturing(&self) -> bool {
        self.is_capturing
    }

    /// Close the AudioContext entirely.
    pub async fn close(&mut self) -> Result<(), JsValue> {
        self.stop()?;
        if let Some(ctx) = self.ctx.take() {
            let _ = wasm_bindgen_futures::JsFuture::from(ctx.close()?).await;
        }
        Ok(())
    }
}

impl Default for AudioCapture {
    fn default() -> Self {
        Self::new()
    }
}

// ── Standalone helpers ──────────────────────────────────────────────

/// Request microphone access and return the MediaStream.
pub async fn get_user_media() -> Result<MediaStream, JsValue> {
    let window = web_sys::window().ok_or_else(|| JsValue::from_str("no window"))?;
    let navigator = window.navigator();
    let media_devices = navigator
        .media_devices()
        .map_err(|_| JsValue::from_str("no media devices"))?;

    let constraints = MediaStreamConstraints::new();
    constraints.set_audio(&JsValue::TRUE);
    constraints.set_video(&JsValue::FALSE);

    let promise = media_devices.get_user_media_with_constraints(&constraints)?;
    let stream = wasm_bindgen_futures::JsFuture::from(promise).await?;
    stream
        .dyn_into::<MediaStream>()
        .map_err(|_| JsValue::from_str("failed to get MediaStream"))
}

/// Register the audio worklet processor from a JS file URL.
pub async fn register_worklet(ctx: &AudioContext, worklet_url: &str) -> Result<(), JsValue> {
    let worklet = ctx.audio_worklet()?;
    let promise = worklet.add_module(worklet_url)?;
    wasm_bindgen_futures::JsFuture::from(promise).await?;
    Ok(())
}
