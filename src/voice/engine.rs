//! Voice engine trait and types

use async_channel::{Receiver, Sender};
use async_trait::async_trait;
use serde::{Deserialize, Serialize};

use crate::error::Result;

/// Trait for voice/conversation AI engines
#[async_trait]
pub trait VoiceEngine: Send + Sync {
    /// Get the engine name
    fn name(&self) -> &'static str;

    /// Initialize a conversation session with a system prompt
    async fn start_session(&mut self, system_prompt: &str) -> Result<()>;

    /// End the current session
    async fn end_session(&mut self) -> Result<()>;

    /// Process incoming audio and get response
    /// Returns the transcript of what was heard and the response
    async fn process_audio(&mut self, audio: &[u8]) -> Result<VoiceResponse>;

    /// Process text input and get audio response
    async fn process_text(&mut self, text: &str) -> Result<VoiceResponse>;

    /// Send audio for the agent to "speak"
    async fn speak(&mut self, text: &str) -> Result<Vec<u8>>;

    /// Get the transcript channel for real-time updates
    fn transcript_channel(&self) -> Option<Receiver<TranscriptEvent>>;

    /// Check if the engine is connected
    fn is_connected(&self) -> bool;

    /// Set the interrupt handler - called when the other party starts speaking
    fn set_interrupt_handler(&mut self, handler: Box<dyn Fn() + Send + Sync>);
}

/// Response from the voice engine
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VoiceResponse {
    /// What was heard (speech-to-text)
    pub transcript_in: Option<String>,

    /// The agent's response text
    pub response_text: Option<String>,

    /// The agent's response audio
    pub response_audio: Option<Vec<u8>>,

    /// Whether the agent needs principal input
    pub needs_input: bool,

    /// Question for the principal (if needs_input is true)
    pub input_prompt: Option<String>,

    /// Tool calls the agent wants to make
    pub tool_calls: Vec<ToolCall>,

    /// Whether the conversation should end
    pub should_end: bool,

    /// Reason for ending (if should_end is true)
    pub end_reason: Option<String>,
}

impl Default for VoiceResponse {
    fn default() -> Self {
        VoiceResponse {
            transcript_in: None,
            response_text: None,
            response_audio: None,
            needs_input: false,
            input_prompt: None,
            tool_calls: Vec::new(),
            should_end: false,
            end_reason: None,
        }
    }
}

/// A tool call requested by the agent
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolCall {
    pub name: String,
    pub arguments: serde_json::Value,
}

/// Real-time transcript event
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TranscriptEvent {
    /// Who is speaking
    pub speaker: Speaker,

    /// The text (may be partial)
    pub text: String,

    /// Whether this is a final transcript or partial
    pub is_final: bool,

    /// Timestamp
    pub timestamp: chrono::DateTime<chrono::Utc>,
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Speaker {
    /// The agent (Karta)
    Agent,
    /// The remote party (person on the phone)
    Remote,
    /// The principal (you)
    Principal,
    /// System message
    System,
}

/// Configuration for the voice session
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VoiceSessionConfig {
    /// System prompt for the AI
    pub system_prompt: String,

    /// Voice to use for TTS
    pub voice: String,

    /// Language
    pub language: String,

    /// Whether to enable interruption handling
    pub enable_interruption: bool,

    /// Sample rate for audio
    pub sample_rate: u32,
}

impl Default for VoiceSessionConfig {
    fn default() -> Self {
        VoiceSessionConfig {
            system_prompt: String::new(),
            voice: "alloy".to_string(),
            language: "en-US".to_string(),
            enable_interruption: true,
            sample_rate: 16000,
        }
    }
}

/// Tool definition for the voice engine
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolDefinition {
    pub name: String,
    pub description: String,
    pub parameters: serde_json::Value,
}

/// Standard tools available to the voice engine
pub fn standard_tools() -> Vec<ToolDefinition> {
    vec![
        ToolDefinition {
            name: "request_principal_input".to_string(),
            description: "Request input from the principal when you need guidance or approval".to_string(),
            parameters: serde_json::json!({
                "type": "object",
                "properties": {
                    "question": {
                        "type": "string",
                        "description": "The question to ask the principal"
                    },
                    "options": {
                        "type": "array",
                        "items": {"type": "string"},
                        "description": "Optional suggested responses"
                    },
                    "urgency": {
                        "type": "string",
                        "enum": ["low", "medium", "high"],
                        "description": "How urgent is this input needed"
                    }
                },
                "required": ["question"]
            }),
        },
        ToolDefinition {
            name: "end_call".to_string(),
            description: "End the current call".to_string(),
            parameters: serde_json::json!({
                "type": "object",
                "properties": {
                    "reason": {
                        "type": "string",
                        "description": "Why the call is ending"
                    },
                    "success": {
                        "type": "boolean",
                        "description": "Whether the task was completed successfully"
                    }
                },
                "required": ["reason", "success"]
            }),
        },
        ToolDefinition {
            name: "note".to_string(),
            description: "Record an important note or piece of information".to_string(),
            parameters: serde_json::json!({
                "type": "object",
                "properties": {
                    "category": {
                        "type": "string",
                        "description": "Category of the note (e.g., 'price', 'date', 'contact')"
                    },
                    "content": {
                        "type": "string",
                        "description": "The note content"
                    }
                },
                "required": ["category", "content"]
            }),
        },
    ]
}
