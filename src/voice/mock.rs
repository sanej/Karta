//! Mock voice engine for testing and development

use async_channel::{bounded, Receiver, Sender};
use async_trait::async_trait;
use std::collections::VecDeque;

use crate::error::Result;
use crate::voice::{TranscriptEvent, ToolCall, VoiceEngine, VoiceResponse, Speaker};

/// Mock voice engine for development/testing
pub struct MockVoiceEngine {
    session_active: bool,
    transcript_tx: Sender<TranscriptEvent>,
    transcript_rx: Receiver<TranscriptEvent>,
    system_prompt: String,
    conversation_history: Vec<(Speaker, String)>,
    scripted_responses: VecDeque<ScriptedResponse>,
    interrupt_handler: Option<Box<dyn Fn() + Send + Sync>>,
}

/// A scripted response for testing
pub struct ScriptedResponse {
    pub response_text: String,
    pub needs_input: bool,
    pub input_prompt: Option<String>,
    pub should_end: bool,
}

impl MockVoiceEngine {
    pub fn new() -> Self {
        let (tx, rx) = bounded(100);
        MockVoiceEngine {
            session_active: false,
            transcript_tx: tx,
            transcript_rx: rx,
            system_prompt: String::new(),
            conversation_history: Vec::new(),
            scripted_responses: VecDeque::new(),
            interrupt_handler: None,
        }
    }

    /// Add a scripted response
    pub fn add_response(&mut self, response: ScriptedResponse) {
        self.scripted_responses.push_back(response);
    }

    /// Set up a mock appointment booking conversation
    pub fn setup_appointment_flow(&mut self) {
        self.scripted_responses = VecDeque::from([
            ScriptedResponse {
                response_text: "Hello, this is Karta calling on behalf of the principal. I'd like to schedule an appointment.".to_string(),
                needs_input: false,
                input_prompt: None,
                should_end: false,
            },
            ScriptedResponse {
                response_text: "Thank you. Would next Tuesday at 2pm work? Let me confirm with the principal.".to_string(),
                needs_input: true,
                input_prompt: Some("They offered Tuesday at 2pm. Does this work?".to_string()),
                should_end: false,
            },
            ScriptedResponse {
                response_text: "Yes, Tuesday at 2pm works perfectly. Could you please send a confirmation?".to_string(),
                needs_input: false,
                input_prompt: None,
                should_end: false,
            },
            ScriptedResponse {
                response_text: "Thank you so much for your help. Have a great day!".to_string(),
                needs_input: false,
                input_prompt: None,
                should_end: true,
            },
        ]);
    }

    /// Set up a mock rental negotiation conversation
    pub fn setup_rental_flow(&mut self) {
        self.scripted_responses = VecDeque::from([
            ScriptedResponse {
                response_text: "Hi, this is Karta calling on behalf of the principal regarding the Oak Street property.".to_string(),
                needs_input: false,
                input_prompt: None,
                should_end: false,
            },
            ScriptedResponse {
                response_text: "I understand the listed price is $2,800. The principal was hoping for something closer to $2,450 with an extended lease term. Would that be possible?".to_string(),
                needs_input: false,
                input_prompt: None,
                should_end: false,
            },
            ScriptedResponse {
                response_text: "They're offering $2,650 for 12 months, or $2,550 for 18 months. Let me check with the principal.".to_string(),
                needs_input: true,
                input_prompt: Some("Counter offer: $2,650/12mo or $2,550/18mo. Your budget ceiling is $2,650. How should I proceed?".to_string()),
                should_end: false,
            },
            ScriptedResponse {
                response_text: "The principal has approved $2,550 for 18 months. We'd like to proceed with the application.".to_string(),
                needs_input: false,
                input_prompt: None,
                should_end: false,
            },
            ScriptedResponse {
                response_text: "Thank you. Please send the application to the principal's email. We look forward to it.".to_string(),
                needs_input: false,
                input_prompt: None,
                should_end: true,
            },
        ]);
    }

    fn get_next_response(&mut self) -> VoiceResponse {
        if let Some(scripted) = self.scripted_responses.pop_front() {
            VoiceResponse {
                transcript_in: None,
                response_text: Some(scripted.response_text),
                response_audio: None,
                needs_input: scripted.needs_input,
                input_prompt: scripted.input_prompt,
                tool_calls: Vec::new(),
                should_end: scripted.should_end,
                end_reason: if scripted.should_end {
                    Some("Task completed".to_string())
                } else {
                    None
                },
            }
        } else {
            // Default response
            VoiceResponse {
                transcript_in: None,
                response_text: Some("I understand. Is there anything else I can help with?".to_string()),
                response_audio: None,
                needs_input: false,
                input_prompt: None,
                tool_calls: Vec::new(),
                should_end: false,
                end_reason: None,
            }
        }
    }
}

impl Default for MockVoiceEngine {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl VoiceEngine for MockVoiceEngine {
    fn name(&self) -> &'static str {
        "mock"
    }

    async fn start_session(&mut self, system_prompt: &str) -> Result<()> {
        self.session_active = true;
        self.system_prompt = system_prompt.to_string();
        self.conversation_history.clear();

        // Send initial system message to transcript
        let event = TranscriptEvent {
            speaker: Speaker::System,
            text: format!("Session started. System: {}",
                system_prompt.chars().take(100).collect::<String>()),
            is_final: true,
            timestamp: chrono::Utc::now(),
        };
        let _ = self.transcript_tx.send(event).await;

        Ok(())
    }

    async fn end_session(&mut self) -> Result<()> {
        self.session_active = false;

        let event = TranscriptEvent {
            speaker: Speaker::System,
            text: "Session ended".to_string(),
            is_final: true,
            timestamp: chrono::Utc::now(),
        };
        let _ = self.transcript_tx.send(event).await;

        Ok(())
    }

    async fn process_audio(&mut self, _audio: &[u8]) -> Result<VoiceResponse> {
        // In mock mode, simulate hearing something and responding
        let mock_heard = "Thank you for calling, how can I help you?";

        // Record what we "heard"
        self.conversation_history.push((Speaker::Remote, mock_heard.to_string()));

        // Send transcript event
        let event = TranscriptEvent {
            speaker: Speaker::Remote,
            text: mock_heard.to_string(),
            is_final: true,
            timestamp: chrono::Utc::now(),
        };
        let _ = self.transcript_tx.send(event).await;

        // Get the next scripted response
        let mut response = self.get_next_response();
        response.transcript_in = Some(mock_heard.to_string());

        // Record our response
        if let Some(ref text) = response.response_text {
            self.conversation_history.push((Speaker::Agent, text.clone()));

            let event = TranscriptEvent {
                speaker: Speaker::Agent,
                text: text.clone(),
                is_final: true,
                timestamp: chrono::Utc::now(),
            };
            let _ = self.transcript_tx.send(event).await;
        }

        Ok(response)
    }

    async fn process_text(&mut self, text: &str) -> Result<VoiceResponse> {
        // Record principal input
        self.conversation_history.push((Speaker::Principal, text.to_string()));

        let event = TranscriptEvent {
            speaker: Speaker::Principal,
            text: text.to_string(),
            is_final: true,
            timestamp: chrono::Utc::now(),
        };
        let _ = self.transcript_tx.send(event).await;

        // Get the next response
        let mut response = self.get_next_response();

        // Record our response
        if let Some(ref resp_text) = response.response_text {
            self.conversation_history.push((Speaker::Agent, resp_text.clone()));

            let event = TranscriptEvent {
                speaker: Speaker::Agent,
                text: resp_text.clone(),
                is_final: true,
                timestamp: chrono::Utc::now(),
            };
            let _ = self.transcript_tx.send(event).await;
        }

        Ok(response)
    }

    async fn speak(&mut self, text: &str) -> Result<Vec<u8>> {
        // Record what we're saying
        self.conversation_history.push((Speaker::Agent, text.to_string()));

        let event = TranscriptEvent {
            speaker: Speaker::Agent,
            text: text.to_string(),
            is_final: true,
            timestamp: chrono::Utc::now(),
        };
        let _ = self.transcript_tx.send(event).await;

        // Return empty audio in mock mode
        Ok(Vec::new())
    }

    fn transcript_channel(&self) -> Option<Receiver<TranscriptEvent>> {
        Some(self.transcript_rx.clone())
    }

    fn is_connected(&self) -> bool {
        self.session_active
    }

    fn set_interrupt_handler(&mut self, handler: Box<dyn Fn() + Send + Sync>) {
        self.interrupt_handler = Some(handler);
    }
}
