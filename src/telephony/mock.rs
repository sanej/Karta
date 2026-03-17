//! Mock telephony provider for testing and development
//!
//! Simulates phone calls without actually making them.

use async_channel::{bounded, Receiver, Sender};
use async_trait::async_trait;
use std::sync::Arc;
use tokio::sync::Mutex;

use crate::error::Result;
use crate::telephony::{AudioChunk, Call, CallState, TelephonyProvider};

/// Mock telephony provider for development/testing
pub struct MockTelephonyProvider {
    from_number: String,
    active_calls: Arc<Mutex<Vec<Call>>>,
}

impl MockTelephonyProvider {
    pub fn new() -> Self {
        MockTelephonyProvider {
            from_number: "+1-555-KARTA".to_string(),
            active_calls: Arc::new(Mutex::new(Vec::new())),
        }
    }
}

impl Default for MockTelephonyProvider {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl TelephonyProvider for MockTelephonyProvider {
    fn name(&self) -> &'static str {
        "mock"
    }

    async fn make_call(&self, to_number: &str) -> Result<Call> {
        let mut call = Call::new(to_number.to_string(), self.from_number.clone());

        // Simulate ringing
        call.state = CallState::Ringing;

        // Simulate connection after a brief delay
        tokio::time::sleep(tokio::time::Duration::from_millis(500)).await;
        call.connect();

        // Store the call
        let mut calls = self.active_calls.lock().await;
        calls.push(call.clone());

        Ok(call)
    }

    async fn end_call(&self, call: &Call) -> Result<()> {
        let mut calls = self.active_calls.lock().await;
        if let Some(c) = calls.iter_mut().find(|c| c.id == call.id) {
            c.end();
        }
        Ok(())
    }

    fn audio_channels(&self, _call: &Call) -> Result<(Receiver<AudioChunk>, Sender<AudioChunk>)> {
        // Create channels for audio streaming
        let (inbound_tx, inbound_rx) = bounded::<AudioChunk>(100);
        let (outbound_tx, outbound_rx) = bounded::<AudioChunk>(100);

        // In mock mode, we just create the channels but don't process audio
        // The voice engine will use these channels
        Ok((inbound_rx, outbound_tx))
    }

    async fn send_audio(&self, _call: &Call, _audio: AudioChunk) -> Result<()> {
        // In mock mode, we just accept the audio but don't do anything with it
        Ok(())
    }

    fn is_ready(&self) -> bool {
        true
    }

    fn from_number(&self) -> &str {
        &self.from_number
    }
}

/// Simulated conversation for testing
pub struct MockConversation {
    /// Scripted responses from the "other party"
    pub responses: Vec<MockResponse>,
    current_index: usize,
}

pub struct MockResponse {
    /// What the mock "hears" (trigger phrase)
    pub trigger: Option<String>,
    /// What the mock responds with
    pub response: String,
    /// Delay before responding (milliseconds)
    pub delay_ms: u64,
}

impl MockConversation {
    /// Create a mock conversation for appointment booking
    pub fn appointment_booking() -> Self {
        MockConversation {
            responses: vec![
                MockResponse {
                    trigger: None,
                    response: "Thank you for calling Acme Medical. How can I help you today?".into(),
                    delay_ms: 1000,
                },
                MockResponse {
                    trigger: Some("appointment".into()),
                    response: "I can help you schedule an appointment. What day works for you?".into(),
                    delay_ms: 500,
                },
                MockResponse {
                    trigger: Some("next week".into()),
                    response: "I have availability on Tuesday at 2pm or Thursday at 10am. Which works better?".into(),
                    delay_ms: 500,
                },
                MockResponse {
                    trigger: Some("tuesday".into()),
                    response: "Great, I've scheduled you for Tuesday at 2pm. Can I get your phone number for confirmation?".into(),
                    delay_ms: 500,
                },
                MockResponse {
                    trigger: None,
                    response: "Perfect. You're all set for Tuesday at 2pm. We'll send a confirmation text. Is there anything else?".into(),
                    delay_ms: 500,
                },
                MockResponse {
                    trigger: Some("no".into()),
                    response: "Thank you for calling. Have a great day!".into(),
                    delay_ms: 500,
                },
            ],
            current_index: 0,
        }
    }

    /// Create a mock conversation for rental inquiry
    pub fn rental_inquiry() -> Self {
        MockConversation {
            responses: vec![
                MockResponse {
                    trigger: None,
                    response: "Apex Properties, this is Sarah speaking. How can I help you?".into(),
                    delay_ms: 1000,
                },
                MockResponse {
                    trigger: Some("rental".into()),
                    response: "Yes, I can help with that. Which unit are you inquiring about?".into(),
                    delay_ms: 500,
                },
                MockResponse {
                    trigger: Some("oak".into()),
                    response: "The unit on Oak Street is $2,800 per month. It's a beautiful 1-bedroom.".into(),
                    delay_ms: 500,
                },
                MockResponse {
                    trigger: Some("2500".into()),
                    response: "I appreciate the offer. The best I can do is $2,650 with a 12-month lease.".into(),
                    delay_ms: 500,
                },
                MockResponse {
                    trigger: Some("18 month".into()),
                    response: "For an 18-month lease, I could do $2,550. Would that work?".into(),
                    delay_ms: 500,
                },
                MockResponse {
                    trigger: Some("yes".into()),
                    response: "Excellent! I'll email the application to you. What email should I use?".into(),
                    delay_ms: 500,
                },
            ],
            current_index: 0,
        }
    }

    /// Get the next response
    pub fn next_response(&mut self) -> Option<&MockResponse> {
        if self.current_index < self.responses.len() {
            let response = &self.responses[self.current_index];
            self.current_index += 1;
            Some(response)
        } else {
            None
        }
    }

    /// Find a response matching the input
    pub fn find_response(&mut self, input: &str) -> Option<&MockResponse> {
        let input_lower = input.to_lowercase();

        // First, find if there's a matching trigger
        let found_index = self.responses[self.current_index..]
            .iter()
            .enumerate()
            .find(|(_, response)| {
                response.trigger.as_ref().map_or(false, |trigger| {
                    input_lower.contains(&trigger.to_lowercase())
                })
            })
            .map(|(i, _)| i);

        if let Some(i) = found_index {
            self.current_index += i + 1;
            return self.responses.get(self.current_index - 1);
        }

        // If no trigger matched, return the next response without a trigger
        if self.current_index < self.responses.len() {
            let response = &self.responses[self.current_index];
            self.current_index += 1;
            Some(response)
        } else {
            None
        }
    }
}
