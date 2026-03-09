use base64::Engine;

/// Encode raw PCM16 bytes to base64 string (for WebSocket transport)
pub fn pcm16_to_base64(pcm_data: &[u8]) -> String {
    base64::engine::general_purpose::STANDARD.encode(pcm_data)
}

/// Decode base64 string back to raw PCM16 bytes
pub fn base64_to_pcm16(encoded: &str) -> Result<Vec<u8>, base64::DecodeError> {
    base64::engine::general_purpose::STANDARD.decode(encoded)
}

/// Convert f32 samples [-1.0, 1.0] to PCM16 little-endian bytes
pub fn f32_to_pcm16(samples: &[f32]) -> Vec<u8> {
    let mut bytes = Vec::with_capacity(samples.len() * 2);
    for &sample in samples {
        let clamped = sample.clamp(-1.0, 1.0);
        let val = (clamped * 32768.0).clamp(-32768.0, 32767.0) as i16;
        bytes.extend_from_slice(&val.to_le_bytes());
    }
    bytes
}

/// Convert PCM16 little-endian bytes to f32 samples [-1.0, 1.0]
pub fn pcm16_to_f32(pcm_data: &[u8]) -> Vec<f32> {
    let mut samples = Vec::with_capacity(pcm_data.len() / 2);
    for chunk in pcm_data.chunks_exact(2) {
        let val = i16::from_le_bytes([chunk[0], chunk[1]]);
        samples.push(val as f32 / 32768.0);
    }
    samples
}

/// Encode f32 samples directly to base64 PCM16
pub fn f32_to_base64(samples: &[f32]) -> String {
    pcm16_to_base64(&f32_to_pcm16(samples))
}

/// Decode base64 PCM16 directly to f32 samples
pub fn base64_to_f32(encoded: &str) -> Result<Vec<f32>, base64::DecodeError> {
    Ok(pcm16_to_f32(&base64_to_pcm16(encoded)?))
}
