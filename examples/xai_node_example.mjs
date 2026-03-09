import WebSocket from 'ws';
// npm install ws · export XAI_API_KEY="xai-..."

const ws = new WebSocket('wss://api.x.ai/v1/realtime', {
  headers: { Authorization: `Bearer ${process.env.XAI_API_KEY}` },
});

ws.on('open', () => {
  ws.send(JSON.stringify({
    type: 'session.update',
    session: {
      voice: 'Eve',
      instructions: "",
      turn_detection: { type: 'server_vad', threshold: 0.85, silence_duration_ms: 0 },
      tools: [{ type: 'web_search' }, { type: 'x_search' }],
      input_audio_transcription: { model: 'grok-2-audio' },
      audio: {
        input: { format: { type: 'audio/pcm', rate: 24000 } },
        output: { format: { type: 'audio/pcm', rate: 24000 } },
      },
    },
  }));
});

ws.on('message', (raw) => {
  const event = JSON.parse(raw.toString());
  switch (event.type) {
    case 'session.created':
      console.log('Session:', event.session.id);
      break;
    case 'input_audio_buffer.speech_started':
      ws.send(JSON.stringify({ type: 'response.cancel' }));
      break;
    case 'response.output_audio.delta':
      const pcm = Buffer.from(event.delta, 'base64');
      break;
    case 'response.output_audio_transcript.delta':
      process.stdout.write(event.delta);
      break;
    case 'response.done':
      console.log('\nDone — tokens:', event.usage?.total_tokens);
      break;
    case 'error':
      console.error('Error:', event.message);
      break;
  }
});

ws.send(JSON.stringify({
  type: 'conversation.item.create',
  item: { type: 'message', role: 'user',
    content: [{ type: 'input_text', text: 'Hello!' }] },
}));
ws.send(JSON.stringify({ type: 'response.create' }));
