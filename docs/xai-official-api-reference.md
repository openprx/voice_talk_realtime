# xAI Voice Agent API - Official Reference Summary

Source: https://docs.x.ai/developers/model-capabilities/audio/voice-agent

## Endpoint
wss://api.x.ai/v1/realtime (us-east-1 only)

## Auth
- Server: Authorization: Bearer <XAI_API_KEY>
- Browser: ephemeral token via POST https://api.x.ai/v1/realtime/client_secrets
  - WebSocket subprotocol: xai-client-secret.<token>

## Voices
Eve (F, default), Ara (F), Rex (M), Sal (N), Leo (M)

## Audio Formats
- audio/pcm: 8000/16000/22050/24000(default)/32000/44100/48000 Hz, 16-bit LE mono
- audio/pcmu: G.711 u-law, 8000 Hz fixed
- audio/pcma: G.711 A-law, 8000 Hz fixed

## Session Config
turn_detection.threshold: 0.0-1.0 (default 0.85)
turn_detection.silence_duration_ms: 100-5000
input_audio_transcription.model: grok-2-audio

## Tools
- web_search, x_search (allowed_x_handles), file_search (vector_store_ids), function (custom JSON schema)
- Function call flow: receive args.done -> execute -> conversation.item.create(function_call_output) -> response.create

## Client Events
session.update, input_audio_buffer.append/commit/clear, conversation.item.create/delete, response.create/cancel

## Server Events
session.created/updated, conversation.created/item.added, input_audio_buffer.speech_started/stopped/committed/cleared,
conversation.item.input_audio_transcription.completed,
response.created/done, response.output_item.added/done, response.output_audio.delta/done,
response.output_audio_transcript.delta/done, response.function_call_arguments.done, error, rate_limits.updated

## Best Practices
1. Parallel init: mic capture + WebSocket connect simultaneously
2. Buffer mic until session.updated (safety cap 8-12s)
3. Avoid audio overlap during tool calls: wait playback complete before response.create
4. Stream output audio deltas instantly
5. Graceful reconnect with continued buffering
