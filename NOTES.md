# Karta Project Notes

## What We Built
- **Bridge server** (Node.js) that connects Twilio phone calls to Gemini Live
- Real phone calls with natural AI voice (not robotic TTS)
- Configurable personality and task via environment variables
- Rust CLI structure (partially built, not connected to bridge yet)

## What Works
- Making outbound calls via Twilio
- Gemini Live voice conversations
- Karta introduces herself as your assistant
- Successfully tested: called yourself, called wife about dinner

## Key Files
- `bridge/server.js` - The working bridge server
- `src/` - Rust CLI (partially built)
- `.env` - Your API keys (not committed)

## How to Run
```bash
# Terminal 1: Bridge server
cd bridge
node server.js

# Terminal 2: Tunnel
cloudflared tunnel --url http://localhost:8080

# Terminal 3: Make call
curl -X POST "https://api.twilio.com/2010-04-01/Accounts/$TWILIO_ACCOUNT_SID/Calls.json" \
  -u "$TWILIO_ACCOUNT_SID:$TWILIO_AUTH_TOKEN" \
  -d "To=+1PHONENUMBER" \
  -d "From=$TWILIO_PHONE_NUMBER" \
  -d "Twiml=<Response><Connect><Stream url=\"wss://YOUR-TUNNEL-URL/media-stream\"/></Connect><Pause length=\"120\"/></Response>"
```

## Your Credentials (in .env)
- Twilio account configured
- Gemini API key configured
- Your Twilio number: +14159121764

## Real Use Cases Identified
1. **Pharmacy calls** - Prescription status, refills, hold times
2. **Rental inquiries** - Availability, pricing across properties
3. **Appointment scheduling** - Places without online booking
4. **Customer service hold** - Insurance, airlines, utilities

## Key Learnings
- Natural voice requires Gemini Live (not TTS)
- Model name: `gemini-2.5-flash-native-audio-preview-12-2025`
- Voice options: Achird (friendly), Sulafat (warm), Aoede (breezy)
- Need to send initial prompt to make Gemini speak first
- Twilio trial accounts play a message before connecting

## What's Not Built Yet
- Dynamic task configuration from CLI
- Real-time text backchannel for supervision
- Call recording/transcripts
- Production server deployment

## Product Reflection
- "Solution looking for a problem?" - Maybe partially
- Daily need is unclear, but pharmacy/rental use cases are real
- Phone anxiety is a genuine underserved market
- Decided to open source and shelve for now

## GitHub
- Repo: https://github.com/sanej/Karta
- Status: Open source, experimental
- Add topics via web UI for discoverability

## To Resume
1. Run the bridge server
2. Start cloudflared tunnel
3. Make test call
4. Pick up from: connecting Rust CLI to bridge for dynamic tasks
