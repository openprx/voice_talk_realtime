/**
 * AudioWorklet processor: captures mic audio, resamples to 24kHz,
 * converts to PCM16 LE, base64-encodes, and posts to the main thread.
 *
 * NOTE: btoa() is NOT available in AudioWorkletGlobalScope.
 * We implement a manual base64 encoder.
 */
class PcmCaptureProcessor extends AudioWorkletProcessor {
  constructor() {
    super();
    this._buffer = [];
    this._chunkSize = 2400; // 100ms at 24kHz
    this._b64chars = 'ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/';
  }

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

  _f32ToPcm16(samples) {
    const buf = new ArrayBuffer(samples.length * 2);
    const view = new DataView(buf);
    for (let i = 0; i < samples.length; i++) {
      let s = Math.max(-1, Math.min(1, samples[i]));
      view.setInt16(i * 2, s * 32767, true);
    }
    return new Uint8Array(buf);
  }

  _uint8ToBase64(bytes) {
    const chars = this._b64chars;
    let result = '';
    const len = bytes.length;
    for (let i = 0; i < len; i += 3) {
      const b0 = bytes[i];
      const b1 = i + 1 < len ? bytes[i + 1] : 0;
      const b2 = i + 2 < len ? bytes[i + 2] : 0;
      result += chars[b0 >> 2];
      result += chars[((b0 & 3) << 4) | (b1 >> 4)];
      result += i + 1 < len ? chars[((b1 & 15) << 2) | (b2 >> 6)] : '=';
      result += i + 2 < len ? chars[b2 & 63] : '=';
    }
    return result;
  }

  process(inputs, outputs, parameters) {
    const input = inputs[0];
    if (!input || !input[0] || input[0].length === 0) return true;

    const channelData = input[0];
    const resampled = this._resample(channelData, sampleRate, 24000);

    for (let i = 0; i < resampled.length; i++) {
      this._buffer.push(resampled[i]);
    }

    while (this._buffer.length >= this._chunkSize) {
      const chunk = new Float32Array(this._buffer.splice(0, this._chunkSize));
      const pcm16 = this._f32ToPcm16(chunk);
      const b64 = this._uint8ToBase64(pcm16);
      this.port.postMessage(b64);
    }

    return true;
  }
}

registerProcessor('pcm-capture-processor', PcmCaptureProcessor);
