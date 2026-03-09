/**
 * AudioWorklet processor: captures microphone audio, resamples to 24kHz,
 * converts to PCM16 LE, base64-encodes, and posts to the main thread.
 */
class PcmCaptureProcessor extends AudioWorkletProcessor {
  constructor() {
    super();
    this._buffer = [];
    // We'll accumulate samples and send chunks periodically
    this._chunkSize = 2400; // 100ms at 24kHz
  }

  /**
   * Resample from source rate to 24000 Hz using linear interpolation.
   */
  _resample(input, fromRate, toRate) {
    if (fromRate === toRate) return input;
    const ratio = fromRate / toRate;
    const outLen = Math.floor(input.length / ratio);
    const output = new Float32Array(outLen);
    for (let i = 0; i < outLen; i++) {
      const srcIdx = i * ratio;
      const lo = Math.floor(srcIdx);
      const hi = Math.min(lo + 1, input.length - 1);
      const frac = srcIdx - lo;
      output[i] = input[lo] * (1 - frac) + input[hi] * frac;
    }
    return output;
  }

  /**
   * Convert Float32 samples [-1,1] to PCM16 LE ArrayBuffer.
   */
  _f32ToPcm16(samples) {
    const buf = new ArrayBuffer(samples.length * 2);
    const view = new DataView(buf);
    for (let i = 0; i < samples.length; i++) {
      let s = Math.max(-1, Math.min(1, samples[i]));
      view.setInt16(i * 2, s * 32767, true); // little-endian
    }
    return buf;
  }

  /**
   * ArrayBuffer to base64.
   */
  _arrayBufferToBase64(buffer) {
    const bytes = new Uint8Array(buffer);
    let binary = '';
    for (let i = 0; i < bytes.length; i++) {
      binary += String.fromCharCode(bytes[i]);
    }
    // Use a chunked btoa approach for large buffers
    return btoa(binary);
  }

  process(inputs, outputs, parameters) {
    const input = inputs[0];
    if (!input || !input[0] || input[0].length === 0) return true;

    const channelData = input[0]; // mono
    const resampled = this._resample(channelData, sampleRate, 24000);

    // Accumulate
    for (let i = 0; i < resampled.length; i++) {
      this._buffer.push(resampled[i]);
    }

    // Send chunks
    while (this._buffer.length >= this._chunkSize) {
      const chunk = new Float32Array(this._buffer.splice(0, this._chunkSize));
      const pcm16 = this._f32ToPcm16(chunk);
      const b64 = this._arrayBufferToBase64(pcm16);
      this.port.postMessage(b64);
    }

    return true;
  }
}

registerProcessor('pcm-capture-processor', PcmCaptureProcessor);
