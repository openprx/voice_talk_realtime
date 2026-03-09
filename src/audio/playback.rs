//! Audio playback: receives base64 PCM16, decodes, and plays through Web Audio API.

use wasm_bindgen::prelude::*;
use web_sys::AudioContext;

use super::codec;

/// Audio playback manager that queues and plays PCM16 audio chunks.
pub struct AudioPlayback {
    ctx: Option<AudioContext>,
    sample_rate: f32,
    /// Tracks the scheduled end time for seamless gapless playback.
    next_play_time: f64,
}

impl AudioPlayback {
    /// Create a new playback manager.
    /// `sample_rate` is the expected PCM sample rate (typically 24000).
    pub fn new(sample_rate: f32) -> Self {
        Self {
            ctx: None,
            sample_rate,
            next_play_time: 0.0,
        }
    }

    /// Initialize (or reuse) the AudioContext.
    pub fn init(&mut self) -> Result<(), JsValue> {
        if self.ctx.is_none() {
            self.ctx = Some(AudioContext::new()?);
        }
        Ok(())
    }

    /// Resume the AudioContext (must be called from user gesture).
    pub async fn resume(&self) -> Result<(), JsValue> {
        if let Some(ctx) = &self.ctx {
            let _ = wasm_bindgen_futures::JsFuture::from(ctx.resume()?).await;
        }
        Ok(())
    }

    /// Enqueue a base64-encoded PCM16 chunk for playback.
    pub fn play_base64_pcm16(&mut self, b64_audio: &str) -> Result<(), JsValue> {
        let ctx = self
            .ctx
            .as_ref()
            .ok_or_else(|| JsValue::from_str("AudioContext not initialized"))?;

        let samples = codec::base64_to_f32(b64_audio)
            .map_err(|e| JsValue::from_str(&format!("{e}")))?;

        if samples.is_empty() {
            return Ok(());
        }

        let buffer = ctx.create_buffer(1, samples.len() as u32, self.sample_rate)?;
        buffer.copy_to_channel(&samples, 0)?;

        let source = ctx.create_buffer_source()?;
        source.set_buffer(Some(&buffer));
        source.connect_with_audio_node(&ctx.destination())?;

        let current_time = ctx.current_time();
        let start_time = if self.next_play_time > current_time {
            self.next_play_time
        } else {
            current_time
        };

        source.start_with_when(start_time)?;
        self.next_play_time = start_time + (samples.len() as f64 / self.sample_rate as f64);

        Ok(())
    }

    /// Stop all scheduled audio and reset.
    pub fn stop(&mut self) -> Result<(), JsValue> {
        self.next_play_time = 0.0;
        // Create a new context to effectively stop all playing sources
        if self.ctx.is_some() {
            self.ctx = Some(AudioContext::new()?);
        }
        Ok(())
    }

    /// Close the AudioContext entirely.
    pub async fn close(&mut self) -> Result<(), JsValue> {
        if let Some(ctx) = self.ctx.take() {
            let _ = wasm_bindgen_futures::JsFuture::from(ctx.close()?).await;
        }
        self.next_play_time = 0.0;
        Ok(())
    }
}

impl Default for AudioPlayback {
    fn default() -> Self {
        Self::new(24000.0)
    }
}
