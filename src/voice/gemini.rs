//! Gemini Live voice engine implementation
//!
//! Integrates with Google's Gemini Live API for real-time voice conversations.

use async_channel::{bounded, Receiver, Sender};
use async_trait::async_trait;
use futures_util::{SinkExt, StreamExt};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tokio::sync::Mutex;
use tokio_tungstenite::{connect_async, tungstenite::Message};

use crate::error::{KartaError, Result};
use crate::voice::{TranscriptEvent, ToolCall, VoiceEngine, VoiceResponse, Speaker};

/// Gemini Live voice engine
pub struct GeminiLiveEngine {
    api_key: String,
    model: String,
    session_active: bool,
    transcript_tx: Sender<TranscriptEvent>,
    transcript_rx: Receiver<TranscriptEvent>,
    ws_sender: Option<Arc<Mutex<futures_util::stream::SplitSink<
        tokio_tungstenite::WebSocketStream<
            tokio_tungstenite::MaybeTlsStream<tokio::net::TcpStream>
        >,
        Message
    >>>>,
    interrupt_handler: Option<Box<dyn Fn() + Send + Sync>>,
}

impl GeminiLiveEngine {
    pub fn new(api_key: String, model: String) -> Self {
        let (tx, rx) = bounded(100);
        GeminiLiveEngine {
            api_key,
            model,
            session_active: false,
            transcript_tx: tx,
            transcript_rx: rx,
            ws_sender: None,
            interrupt_handler: None,
        }
    }

    /// Get the WebSocket URL for Gemini Live
    fn websocket_url(&self) -> String {
        format!(
            "wss://generativelanguage.googleapis.com/ws/google.ai.generativelanguage.v1beta.GenerativeService.BidiGenerateContent?key={}",
            self.api_key
        )
    }

    /// Send a message through the WebSocket
    async fn send_message(&self, msg: GeminiMessage) -> Result<()> {
        if let Some(ref sender) = self.ws_sender {
            let json = serde_json::to_string(&msg)?;
            let mut sender = sender.lock().await;
            sender.send(Message::Text(json)).await
                .map_err(|e| KartaError::VoiceEngine(format!("WebSocket send error: {}", e)))?;
        }
        Ok(())
    }

    /// Process a response from Gemini
    fn process_gemini_response(&self, response: &GeminiResponse) -> VoiceResponse {
        let mut voice_response = VoiceResponse::default();

        // Extract transcript from the response
        if let Some(ref server_content) = response.server_content {
            if let Some(ref parts) = server_content.model_turn {
                for part in &parts.parts {
                    if let Some(ref text) = part.text {
                        voice_response.response_text = Some(text.clone());
                    }
                    if let Some(ref inline_data) = part.inline_data {
                        if inline_data.mime_type.starts_with("audio/") {
                            // Decode base64 audio
                            if let Ok(audio) = base64_decode(&inline_data.data) {
                                voice_response.response_audio = Some(audio);
                            }
                        }
                    }
                }
            }
        }

        // Check for tool calls
        if let Some(ref tool_call) = response.tool_call {
            for fc in &tool_call.function_calls {
                voice_response.tool_calls.push(ToolCall {
                    name: fc.name.clone(),
                    arguments: fc.args.clone(),
                });

                // Check if it's a request for principal input
                if fc.name == "request_principal_input" {
                    voice_response.needs_input = true;
                    if let Some(question) = fc.args.get("question").and_then(|q| q.as_str()) {
                        voice_response.input_prompt = Some(question.to_string());
                    }
                }

                // Check if it's an end call request
                if fc.name == "end_call" {
                    voice_response.should_end = true;
                    if let Some(reason) = fc.args.get("reason").and_then(|r| r.as_str()) {
                        voice_response.end_reason = Some(reason.to_string());
                    }
                }
            }
        }

        voice_response
    }
}

#[async_trait]
impl VoiceEngine for GeminiLiveEngine {
    fn name(&self) -> &'static str {
        "gemini-live"
    }

    async fn start_session(&mut self, system_prompt: &str) -> Result<()> {
        let url = self.websocket_url();

        // Connect to WebSocket
        let (ws_stream, _) = connect_async(&url).await
            .map_err(|e| KartaError::Connection(format!("WebSocket connection failed: {}", e)))?;

        let (sender, mut receiver) = ws_stream.split();
        self.ws_sender = Some(Arc::new(Mutex::new(sender)));

        // Send setup message
        let setup = GeminiMessage::Setup(GeminiSetup {
            model: format!("models/{}", self.model),
            generation_config: GenerationConfig {
                response_modalities: vec!["AUDIO".to_string(), "TEXT".to_string()],
                speech_config: Some(SpeechConfig {
                    voice_config: VoiceConfig {
                        prebuilt_voice_config: PrebuiltVoiceConfig {
                            voice_name: "Aoede".to_string(),
                        },
                    },
                }),
            },
            system_instruction: Some(Content {
                parts: vec![Part {
                    text: Some(system_prompt.to_string()),
                    inline_data: None,
                }],
            }),
            tools: Some(vec![Tool {
                function_declarations: crate::voice::standard_tools()
                    .into_iter()
                    .map(|t| FunctionDeclaration {
                        name: t.name,
                        description: t.description,
                        parameters: Some(t.parameters),
                    })
                    .collect(),
            }]),
        });

        self.send_message(setup).await?;

        self.session_active = true;

        // Spawn a task to handle incoming messages
        let transcript_tx = self.transcript_tx.clone();
        tokio::spawn(async move {
            while let Some(msg) = receiver.next().await {
                match msg {
                    Ok(Message::Text(text)) => {
                        if let Ok(response) = serde_json::from_str::<GeminiResponse>(&text) {
                            // Extract transcript and send to channel
                            if let Some(ref server_content) = response.server_content {
                                if let Some(ref parts) = server_content.model_turn {
                                    for part in &parts.parts {
                                        if let Some(ref text) = part.text {
                                            let event = TranscriptEvent {
                                                speaker: Speaker::Agent,
                                                text: text.clone(),
                                                is_final: server_content.turn_complete.unwrap_or(false),
                                                timestamp: chrono::Utc::now(),
                                            };
                                            let _ = transcript_tx.send(event).await;
                                        }
                                    }
                                }
                            }
                        }
                    }
                    Ok(Message::Close(_)) => break,
                    Err(e) => {
                        tracing::error!("WebSocket error: {}", e);
                        break;
                    }
                    _ => {}
                }
            }
        });

        Ok(())
    }

    async fn end_session(&mut self) -> Result<()> {
        self.session_active = false;
        self.ws_sender = None;
        Ok(())
    }

    async fn process_audio(&mut self, audio: &[u8]) -> Result<VoiceResponse> {
        if !self.session_active {
            return Err(KartaError::VoiceEngine("Session not active".into()));
        }

        // Send audio to Gemini
        let audio_b64 = base64_encode(audio);
        let msg = GeminiMessage::RealtimeInput(RealtimeInput {
            media_chunks: vec![MediaChunk {
                mime_type: "audio/pcm;rate=16000".to_string(),
                data: audio_b64,
            }],
        });

        self.send_message(msg).await?;

        // For now, return empty response - actual response comes via WebSocket
        Ok(VoiceResponse::default())
    }

    async fn process_text(&mut self, text: &str) -> Result<VoiceResponse> {
        if !self.session_active {
            return Err(KartaError::VoiceEngine("Session not active".into()));
        }

        // Send text as client content
        let msg = GeminiMessage::ClientContent(ClientContent {
            turns: vec![Turn {
                role: "user".to_string(),
                parts: vec![Part {
                    text: Some(text.to_string()),
                    inline_data: None,
                }],
            }],
            turn_complete: true,
        });

        self.send_message(msg).await?;

        Ok(VoiceResponse::default())
    }

    async fn speak(&mut self, text: &str) -> Result<Vec<u8>> {
        // For TTS, we'd send text and receive audio
        // This is a simplified version
        self.process_text(text).await?;
        Ok(Vec::new()) // Audio would come via WebSocket
    }

    fn transcript_channel(&self) -> Option<Receiver<TranscriptEvent>> {
        Some(self.transcript_rx.clone())
    }

    fn is_connected(&self) -> bool {
        self.session_active && self.ws_sender.is_some()
    }

    fn set_interrupt_handler(&mut self, handler: Box<dyn Fn() + Send + Sync>) {
        self.interrupt_handler = Some(handler);
    }
}

// Gemini API message types

#[derive(Debug, Serialize)]
#[serde(untagged)]
enum GeminiMessage {
    Setup(GeminiSetup),
    ClientContent(ClientContent),
    RealtimeInput(RealtimeInput),
    ToolResponse(ToolResponse),
}

#[derive(Debug, Serialize)]
struct GeminiSetup {
    model: String,
    generation_config: GenerationConfig,
    #[serde(skip_serializing_if = "Option::is_none")]
    system_instruction: Option<Content>,
    #[serde(skip_serializing_if = "Option::is_none")]
    tools: Option<Vec<Tool>>,
}

#[derive(Debug, Serialize)]
struct GenerationConfig {
    response_modalities: Vec<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    speech_config: Option<SpeechConfig>,
}

#[derive(Debug, Serialize)]
struct SpeechConfig {
    voice_config: VoiceConfig,
}

#[derive(Debug, Serialize)]
struct VoiceConfig {
    prebuilt_voice_config: PrebuiltVoiceConfig,
}

#[derive(Debug, Serialize)]
struct PrebuiltVoiceConfig {
    voice_name: String,
}

#[derive(Debug, Serialize)]
struct Content {
    parts: Vec<Part>,
}

#[derive(Debug, Serialize, Deserialize)]
struct Part {
    #[serde(skip_serializing_if = "Option::is_none")]
    text: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    inline_data: Option<InlineData>,
}

#[derive(Debug, Serialize, Deserialize)]
struct InlineData {
    mime_type: String,
    data: String,
}

#[derive(Debug, Serialize)]
struct Tool {
    function_declarations: Vec<FunctionDeclaration>,
}

#[derive(Debug, Serialize)]
struct FunctionDeclaration {
    name: String,
    description: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    parameters: Option<serde_json::Value>,
}

#[derive(Debug, Serialize)]
struct ClientContent {
    turns: Vec<Turn>,
    turn_complete: bool,
}

#[derive(Debug, Serialize)]
struct Turn {
    role: String,
    parts: Vec<Part>,
}

#[derive(Debug, Serialize)]
struct RealtimeInput {
    media_chunks: Vec<MediaChunk>,
}

#[derive(Debug, Serialize)]
struct MediaChunk {
    mime_type: String,
    data: String,
}

#[derive(Debug, Serialize)]
struct ToolResponse {
    function_responses: Vec<FunctionResponse>,
}

#[derive(Debug, Serialize)]
struct FunctionResponse {
    name: String,
    response: serde_json::Value,
}

// Response types

#[derive(Debug, Deserialize)]
struct GeminiResponse {
    #[serde(default)]
    server_content: Option<ServerContent>,
    #[serde(default)]
    tool_call: Option<ToolCallResponse>,
}

#[derive(Debug, Deserialize)]
struct ServerContent {
    #[serde(default)]
    model_turn: Option<ModelTurn>,
    #[serde(default)]
    turn_complete: Option<bool>,
}

#[derive(Debug, Deserialize)]
struct ModelTurn {
    parts: Vec<Part>,
}

#[derive(Debug, Deserialize)]
struct ToolCallResponse {
    function_calls: Vec<FunctionCall>,
}

#[derive(Debug, Deserialize)]
struct FunctionCall {
    name: String,
    args: serde_json::Value,
}

// Helper functions

use base64::{Engine as _, engine::general_purpose::STANDARD as BASE64_STANDARD};

fn base64_encode(data: &[u8]) -> String {
    BASE64_STANDARD.encode(data)
}

fn base64_decode(data: &str) -> std::result::Result<Vec<u8>, base64::DecodeError> {
    BASE64_STANDARD.decode(data)
}
