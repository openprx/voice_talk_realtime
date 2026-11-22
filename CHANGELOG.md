# Changelog

All notable changes to this project are recorded here, newest first. The format follows
[Keep a Changelog](https://keepachangelog.com/en/1.1.0/). The project has no released versions yet; entries
are dated and summarise the git history.

## [Unreleased]

### Changed

- The README and the package manifests now state the Apache-2.0 licence, matching the `LICENSE` file.

### Removed

- 2026-11-22: the original project plan (`PLAN.md`) and the notes on the vendor's API page
  (`docs/xai-official-api-reference.md`) left the repository; they were working material, not documentation of the library.
- 2026-11-22: the vendor integration guide (`XAI_REALTIME_API_GUIDE.md`) and the standalone Node example
  (`examples/xai_node_example.mjs`); neither belongs to this library, and the example did not work.

### Changed

- 2026-11-22: `.gitignore` no longer carries a malformed `-e /target/` line (an artefact of `echo -e`); `/target/` was already listed.

## 2026-03-18

### Added

- An Apache-2.0 `LICENSE` file.

## 2026-03-10

### Added

- The OpenPRX WIT Component Model plugin (`prx-plugin-wit/`).
- The OpenAI voices `ballad` and `verse` in the web demo.

### Changed

- The xAI client returns an error when API-key authentication is selected, instead of logging a warning and continuing:
  a browser WebSocket cannot send custom headers, so that mode cannot work there.
- The web demo's capture `AudioContext` uses the browser's default sample rate. Forcing 24 kHz made
  `createMediaStreamSource` fail in some browsers; the audio worklet already resamples to 24 kHz.

### Fixed

- Microphone start-up in the web demo: failure of `getUserMedia` shows an error and allows a retry, the stream is
  checked before `createMediaStreamSource`, a suspended `AudioContext` is resumed (Safari), and echo cancellation and
  noise suppression are enabled.
- Switching providers in the web demo closes the old WebSocket first, which removed spurious errors from the stale
  connection; the `AudioContext` is created after `getUserMedia` returns a valid stream, and the button text follows
  the VAD or push-to-talk mode after an error.

### Removed

- Certificate files from the repository (now ignored) and a local guide file (now ignored).

## 2026-03-09

### Added

- The OpenAI Realtime API WebSocket client for WASM (`RealtimeClient` trait, client events and server events).
- The xAI Realtime API client with its five voices (Eve, Ara, Rex, Sal, Leo) and the `web_search` and `x_search` tools.
- The audio pipeline: PCM16, `f32` and base64 conversion; microphone capture through `getUserMedia` and an
  AudioWorklet; PCM playback through an `AudioContext`.
- WASM bindings (`VoiceTalkClient`, the codec helpers) and a web demo with provider selection, API-key input,
  push-to-talk, a VAD mode toggle, audio visualisation and a transcript.
- The OpenPRX WASM plugin (`prx-plugin/`), which handles signalling only.
- A README with the architecture, build instructions and usage.
- `cancel_response()` and `clear_audio()` on the clients, exposed in the WASM bindings together with `send_text`.

### Changed

- `send_text` creates a conversation item and then requests a response, instead of setting response instructions.
- `append_audio_base64` only appends; the server's voice-activity detection commits the audio.
- Incoming xAI events are matched by name before the generic parse, so `output_audio.delta` is no longer
  swallowed as an unknown event.
- The xAI endpoint no longer carries a `?model=` parameter, and the undocumented `xai-insecure-api-key`
  subprotocol was dropped; API-key mode warns that a client secret is needed in the browser.

### Fixed

- Plugin memory layout deallocation, `result_len()` and null VAD handling for xAI in `prx-plugin`.
- Audit findings in the client: two critical, four high and five medium issues, and the `RealtimeClient` trait
  aligned with its implementation.
