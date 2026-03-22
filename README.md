# Karta

**AI Voice Assistant that makes phone calls for you**

Karta connects Twilio phone calls to Google's Gemini Live API, enabling natural voice conversations. Your AI assistant can make calls on your behalf - booking appointments, asking questions, or handling routine phone tasks.

## Status: Experimental / Proof of Concept

This is a working prototype. It successfully:
- Makes real phone calls via Twilio
- Uses Gemini Live for natural voice (not robotic TTS)
- Introduces itself as your personal assistant
- Has basic conversation abilities

**What's NOT built yet:**
- Dynamic task configuration via CLI
- Real-time text backchannel for supervision
- Call recording/transcripts
- Production deployment

## Demo

https://github.com/user-attachments/assets/demo-placeholder

*Karta calling to ask about dinner plans*

## How It Works

```
Your Phone <-- Twilio <-- Bridge Server <-- Gemini Live AI
                              |
                         (WebSocket)
```

1. Bridge server connects Twilio Media Streams to Gemini Live
2. Audio flows bidirectionally in real-time
3. Gemini handles the conversation with your configured personality

## Quick Start

### Prerequisites
- Node.js 18+
- Twilio account with a phone number
- Google AI API key (for Gemini)
- A way to expose localhost (ngrok, cloudflared, etc.)

### Setup

```bash
# Clone the repo
git clone https://github.com/sanej/Karta.git
cd karta

# Set up the bridge server
cd bridge
npm install

# Configure environment
cp ../.env.example .env
# Edit .env with your API keys
```

### Configure `.env`

```bash
# Twilio (https://console.twilio.com)
TWILIO_ACCOUNT_SID=ACxxxxxxxx
TWILIO_AUTH_TOKEN=your_token
TWILIO_PHONE_NUMBER=+15550000000

# Gemini (https://aistudio.google.com/apikey)
GEMINI_API_KEY=your_api_key
```

### Run

```bash
# Terminal 1: Start the bridge server
cd bridge
node server.js

# Terminal 2: Expose to internet (pick one)
cloudflared tunnel --url http://localhost:8080
# or: ngrok http 8080

# Terminal 3: Make a call
curl -X POST "https://api.twilio.com/2010-04-01/Accounts/$TWILIO_ACCOUNT_SID/Calls.json" \
  -u "$TWILIO_ACCOUNT_SID:$TWILIO_AUTH_TOKEN" \
  -d "To=+1YOURNUMBER" \
  -d "From=$TWILIO_PHONE_NUMBER" \
  -d "Twiml=<Response><Connect><Stream url=\"wss://YOUR-TUNNEL-URL/media-stream\"/></Connect><Pause length=\"120\"/></Response>"
```

## Customization

Edit `bridge/server.js` to change:

```javascript
// Your name
const PRINCIPAL_NAME = "Your Name";

// What Karta should do on the call
const TASK = "Call to ask what we're having for dinner tonight.";

// Voice (options: Achird, Sulafat, Aoede, Puck, Charon, etc.)
voiceName: 'Achird'  // "Friendly" voice
```

## Architecture

```
karta/
├── bridge/              # Node.js WebSocket bridge (THIS IS THE WORKING PART)
│   ├── server.js        # Twilio <-> Gemini Live bridge
│   └── package.json
├── src/                 # Rust CLI (partially built, not connected)
│   ├── main.rs
│   ├── cli.rs
│   ├── config/          # Principal profile system
│   ├── telephony/       # Phone providers
│   └── voice/           # Voice engines
└── .env.example         # Environment template
```

## Known Limitations

- **Twilio trial accounts** play a message before connecting ("Press any key...")
- **Latency** depends on your tunnel - production server would be faster
- **No interruption handling** - basic turn-taking only
- **Hardcoded config** - task/personality must be edited in code

## Future Ideas

- [ ] CLI to configure tasks dynamically
- [ ] Real-time transcript in terminal
- [ ] "Backchannel" - text commands to steer the conversation
- [ ] Multiple voice providers (OpenAI, ElevenLabs)
- [ ] Call recording and summaries
- [ ] Deploy as a service

## Use Cases

Real situations where this helps:

- **Pharmacy calls** - Checking prescription status, refills, hold times
- **Rental inquiries** - Asking about availability, pricing, amenities across multiple properties
- **Appointment scheduling** - Doctors, dentists, services that don't have online booking
- **Customer service hold hell** - Insurance, airlines, utilities
- **Phone anxiety** - Some people genuinely struggle with phone calls

## Why This Exists

Built to explore: "Can an AI make routine phone calls for me?"

Tested successfully for real calls. The need isn't daily, but when you DO need it (pharmacy hold, rental hunting), it's genuinely useful.

## Contributing

This is an experimental project. Feel free to:
- Open issues with ideas or bugs
- Submit PRs
- Fork and build your own version

## Contact

- **GitHub Issues**: Best for bugs and feature ideas
- **Discussions**: For questions and ideas
- **Twitter/X**: [@sanejb](https://twitter.com/sanejb)

## License

MIT

---

*Built with Claude Code, Twilio, and Gemini Live*
