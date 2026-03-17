/**
 * Karta Bridge Server
 *
 * Bridges Twilio Media Streams <-> Gemini Live for real-time voice AI
 *
 * Audio flow:
 * - Twilio sends: 8kHz mulaw audio
 * - We convert to: 16kHz 16-bit PCM for Gemini
 * - Gemini sends: 24kHz PCM audio
 * - We convert to: 8kHz mulaw for Twilio
 */

import { WebSocketServer, WebSocket } from 'ws';
import http from 'http';
import dotenv from 'dotenv';

dotenv.config({ path: '../.env' });

const PORT = process.env.PORT || 8080;
const GEMINI_API_KEY = process.env.GEMINI_API_KEY;
const GEMINI_MODEL = 'gemini-2.5-flash-native-audio-preview-12-2025';

// System prompt for Karta - customize these for your use case
const PRINCIPAL_NAME = process.env.PRINCIPAL_NAME || "Your Name";
const TASK = process.env.TASK || "This is a test call to check if the assistant is working.";

const SYSTEM_PROMPT = `You are Karta, a friendly personal assistant for ${PRINCIPAL_NAME}.

YOUR TASK ON THIS CALL:
${TASK}

CRITICAL VOICE GUIDELINES:
- Speak like a real human on a phone call, NOT like an AI or robot
- Use casual, natural language with occasional fillers like "um", "let's see", "so"
- Keep sentences SHORT - phone conversations use brief phrases
- Sound warm, friendly, and relaxed - like calling a friend
- React naturally: "Oh great!", "Perfect!", "Hmm, let me think..."

WHO YOU ARE:
- You're Karta, personal assistant for ${PRINCIPAL_NAME}
- Introduce yourself: "Hi! This is Karta, I'm ${PRINCIPAL_NAME}'s assistant"
- You're helpful, efficient, and friendly

HOW TO TALK:
- Start with a friendly greeting and introduce yourself
- State why you're calling briefly
- Be conversational - ask questions naturally
- Listen and respond like a real person would
- Keep responses short - this is a phone call, not an essay

NEVER:
- Sound robotic or like an automated system
- Use corporate or overly formal language
- Give long monologues
- Forget to mention who you work for`;

// Create HTTP server for health checks and TwiML
const httpServer = http.createServer((req, res) => {
  if (req.url === '/health') {
    res.writeHead(200, { 'Content-Type': 'application/json' });
    res.end(JSON.stringify({ status: 'ok' }));
    return;
  }

  if (req.url === '/twiml') {
    // Return TwiML that connects to our WebSocket
    const host = req.headers.host;
    const wsUrl = `wss://${host}/media-stream`;

    res.writeHead(200, { 'Content-Type': 'application/xml' });
    res.end(`<?xml version="1.0" encoding="UTF-8"?>
<Response>
  <Connect>
    <Stream url="${wsUrl}">
      <Parameter name="caller" value="karta"/>
    </Stream>
  </Connect>
  <Pause length="60"/>
</Response>`);
    return;
  }

  res.writeHead(404);
  res.end('Not found');
});

// Create WebSocket server
const wss = new WebSocketServer({ server: httpServer });

console.log('🚀 Karta Bridge Server starting...');
console.log(`   Gemini API Key: ${GEMINI_API_KEY ? '✓ configured' : '✗ missing'}`);

wss.on('connection', (twilioWs, req) => {
  console.log('📞 New Twilio connection from:', req.url);

  let geminiWs = null;
  let streamSid = null;
  let callSid = null;

  // Connect to Gemini Live
  const connectToGemini = async () => {
    const geminiUrl = `wss://generativelanguage.googleapis.com/ws/google.ai.generativelanguage.v1beta.GenerativeService.BidiGenerateContent?key=${GEMINI_API_KEY}`;

    geminiWs = new WebSocket(geminiUrl);

    geminiWs.on('open', () => {
      console.log('🤖 Connected to Gemini Live');

      // Send setup message (must use camelCase per API spec)
      const setup = {
        setup: {
          model: `models/${GEMINI_MODEL}`,
          generationConfig: {
            responseModalities: ['AUDIO'],
            speechConfig: {
              voiceConfig: {
                prebuiltVoiceConfig: {
                  voiceName: 'Achird'
                }
              }
            }
          },
          systemInstruction: {
            parts: [{ text: SYSTEM_PROMPT }]
          }
        }
      };

      geminiWs.send(JSON.stringify(setup));
      console.log('📤 Sent Gemini setup:', JSON.stringify(setup, null, 2));
    });

    geminiWs.on('message', (data) => {
      try {
        const response = JSON.parse(data.toString());
        console.log('📥 Gemini message:', JSON.stringify(response).substring(0, 200));

        // Handle setup complete - trigger Gemini to start talking
        if (response.setupComplete) {
          console.log('✓ Gemini setup complete');

          // Send initial prompt to make Gemini speak first
          const initialPrompt = {
            clientContent: {
              turns: [{
                role: "user",
                parts: [{ text: "The phone call has just connected. Please greet the person who answered." }]
              }],
              turnComplete: true
            }
          };
          geminiWs.send(JSON.stringify(initialPrompt));
          console.log('📤 Sent initial prompt to start conversation');
          return;
        }

        // Handle audio response from Gemini
        if (response.serverContent?.modelTurn?.parts) {
          for (const part of response.serverContent.modelTurn.parts) {
            if (part.inlineData?.mimeType?.startsWith('audio/')) {
              // Convert and send audio back to Twilio
              const audioData = part.inlineData.data; // base64 PCM 24kHz

              // Convert 24kHz PCM to 8kHz mulaw for Twilio
              const mulawAudio = convertPcmToMulaw(audioData);

              if (streamSid && twilioWs.readyState === WebSocket.OPEN) {
                console.log(`🔊 Sending ${mulawAudio.length} bytes of audio to Twilio`);
                twilioWs.send(JSON.stringify({
                  event: 'media',
                  streamSid: streamSid,
                  media: {
                    payload: mulawAudio
                  }
                }));
              } else {
                console.log(`⚠️ Cannot send audio: streamSid=${streamSid}, wsState=${twilioWs.readyState}`);
              }
            }

            if (part.text) {
              console.log('🤖 Gemini:', part.text);
            }
          }
        }

        if (response.serverContent?.turnComplete) {
          console.log('✓ Gemini turn complete');
        }

      } catch (e) {
        console.error('Error parsing Gemini response:', e);
      }
    });

    geminiWs.on('error', (err) => {
      console.error('❌ Gemini WebSocket error:', err.message);
    });

    geminiWs.on('close', (code, reason) => {
      console.log(`🔌 Gemini connection closed: code=${code}, reason=${reason.toString()}`);
    });
  };

  // Handle messages from Twilio
  twilioWs.on('message', async (message) => {
    try {
      const data = JSON.parse(message.toString());

      switch (data.event) {
        case 'connected':
          console.log('📱 Twilio connected');
          break;

        case 'start':
          streamSid = data.start.streamSid;
          callSid = data.start.callSid;
          console.log(`📱 Stream started: ${streamSid}`);

          // Connect to Gemini when call starts
          await connectToGemini();
          break;

        case 'media':
          // Forward audio to Gemini
          if (geminiWs && geminiWs.readyState === WebSocket.OPEN) {
            const audioPayload = data.media.payload; // base64 mulaw 8kHz

            // Convert 8kHz mulaw to 16kHz PCM for Gemini
            const pcmAudio = convertMulawToPcm(audioPayload);

            geminiWs.send(JSON.stringify({
              realtime_input: {
                media_chunks: [{
                  mime_type: 'audio/pcm;rate=16000',
                  data: pcmAudio
                }]
              }
            }));
          }
          break;

        case 'stop':
          console.log('📱 Stream stopped');
          if (geminiWs) {
            geminiWs.close();
          }
          break;

        default:
          console.log('Unknown event:', data.event);
      }

    } catch (e) {
      console.error('Error handling Twilio message:', e);
    }
  });

  twilioWs.on('close', () => {
    console.log('📱 Twilio connection closed');
    if (geminiWs) {
      geminiWs.close();
    }
  });

  twilioWs.on('error', (err) => {
    console.error('Twilio WebSocket error:', err);
  });
});

// Audio conversion functions
// Note: These are simplified - production would use proper audio libraries

function convertMulawToPcm(mulawBase64) {
  // Decode base64
  const mulawBuffer = Buffer.from(mulawBase64, 'base64');

  // Mulaw to PCM conversion table
  const mulawToPcmTable = new Int16Array(256);
  for (let i = 0; i < 256; i++) {
    let mulaw = ~i;
    let sign = mulaw & 0x80;
    let exponent = (mulaw >> 4) & 0x07;
    let mantissa = mulaw & 0x0F;
    let sample = ((mantissa << 3) + 0x84) << (exponent);
    sample -= 0x84;
    mulawToPcmTable[i] = sign ? -sample : sample;
  }

  // Convert mulaw to 16-bit PCM
  const pcmBuffer = Buffer.alloc(mulawBuffer.length * 2);
  for (let i = 0; i < mulawBuffer.length; i++) {
    const sample = mulawToPcmTable[mulawBuffer[i]];
    pcmBuffer.writeInt16LE(sample, i * 2);
  }

  // Upsample from 8kHz to 16kHz (simple linear interpolation)
  const upsampledBuffer = Buffer.alloc(pcmBuffer.length * 2);
  for (let i = 0; i < pcmBuffer.length / 2; i++) {
    const sample = pcmBuffer.readInt16LE(i * 2);
    const nextSample = i < pcmBuffer.length / 2 - 1
      ? pcmBuffer.readInt16LE((i + 1) * 2)
      : sample;

    upsampledBuffer.writeInt16LE(sample, i * 4);
    upsampledBuffer.writeInt16LE(Math.round((sample + nextSample) / 2), i * 4 + 2);
  }

  return upsampledBuffer.toString('base64');
}

function convertPcmToMulaw(pcmBase64) {
  // Decode base64
  const pcmBuffer = Buffer.from(pcmBase64, 'base64');

  // Downsample from 24kHz to 8kHz (take every 3rd sample)
  const downsampledLength = Math.floor(pcmBuffer.length / 6);
  const downsampledBuffer = Buffer.alloc(downsampledLength * 2);

  for (let i = 0; i < downsampledLength; i++) {
    const srcIndex = i * 6;
    if (srcIndex + 1 < pcmBuffer.length) {
      const sample = pcmBuffer.readInt16LE(srcIndex);
      downsampledBuffer.writeInt16LE(sample, i * 2);
    }
  }

  // PCM to mulaw conversion
  const mulawBuffer = Buffer.alloc(downsampledLength);

  const MULAW_MAX = 0x1FFF;
  const MULAW_BIAS = 33;

  for (let i = 0; i < downsampledLength; i++) {
    let sample = downsampledBuffer.readInt16LE(i * 2);

    // Get sign
    let sign = (sample < 0) ? 0x80 : 0;
    if (sign) sample = -sample;

    // Add bias
    sample += MULAW_BIAS;

    // Clip
    if (sample > MULAW_MAX) sample = MULAW_MAX;

    // Find exponent and mantissa
    let exponent = 7;
    for (let expMask = 0x4000; exponent > 0 && !(sample & expMask); exponent--, expMask >>= 1);

    let mantissa = (sample >> (exponent + 3)) & 0x0F;
    let mulaw = ~(sign | (exponent << 4) | mantissa);

    mulawBuffer[i] = mulaw & 0xFF;
  }

  return mulawBuffer.toString('base64');
}

// Start server
httpServer.listen(PORT, () => {
  console.log(`\n🎯 Karta Bridge Server running on port ${PORT}`);
  console.log(`   Health check: http://localhost:${PORT}/health`);
  console.log(`   TwiML endpoint: http://localhost:${PORT}/twiml`);
  console.log(`\n📌 Next steps:`);
  console.log(`   1. Run: ngrok http ${PORT}`);
  console.log(`   2. Update Twilio to use: https://<ngrok-url>/twiml`);
  console.log(`   3. Make a call!`);
});
