//! Call representation and state

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// Represents an active or completed phone call
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Call {
    /// Unique call identifier
    pub id: Uuid,

    /// The phone number being called
    pub to_number: String,

    /// The phone number calling from
    pub from_number: String,

    /// Current state of the call
    pub state: CallState,

    /// When the call was initiated
    pub initiated_at: DateTime<Utc>,

    /// When the call was answered (if applicable)
    pub answered_at: Option<DateTime<Utc>>,

    /// When the call ended (if applicable)
    pub ended_at: Option<DateTime<Utc>>,

    /// Duration in seconds (if ended)
    pub duration_seconds: Option<u32>,

    /// Provider-specific call ID
    pub provider_call_id: Option<String>,
}

/// State of a phone call
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum CallState {
    /// Call is being initiated
    Initiating,
    /// Phone is ringing
    Ringing,
    /// Call is connected and active
    Connected,
    /// Call is on hold
    OnHold,
    /// Call ended normally
    Ended,
    /// Call failed to connect
    Failed(String),
    /// Call was not answered
    NoAnswer,
    /// Line was busy
    Busy,
}

impl Call {
    /// Create a new call
    pub fn new(to_number: String, from_number: String) -> Self {
        Call {
            id: Uuid::new_v4(),
            to_number,
            from_number,
            state: CallState::Initiating,
            initiated_at: Utc::now(),
            answered_at: None,
            ended_at: None,
            duration_seconds: None,
            provider_call_id: None,
        }
    }

    /// Mark the call as connected
    pub fn connect(&mut self) {
        self.state = CallState::Connected;
        self.answered_at = Some(Utc::now());
    }

    /// End the call
    pub fn end(&mut self) {
        self.state = CallState::Ended;
        self.ended_at = Some(Utc::now());

        if let Some(answered) = self.answered_at {
            let duration = Utc::now().signed_duration_since(answered);
            self.duration_seconds = Some(duration.num_seconds() as u32);
        }
    }

    /// Mark the call as failed
    pub fn fail(&mut self, reason: String) {
        self.state = CallState::Failed(reason);
        self.ended_at = Some(Utc::now());
    }

    /// Check if the call is active
    pub fn is_active(&self) -> bool {
        matches!(self.state, CallState::Connected | CallState::OnHold | CallState::Ringing)
    }

    /// Check if the call has ended
    pub fn is_ended(&self) -> bool {
        matches!(
            self.state,
            CallState::Ended | CallState::Failed(_) | CallState::NoAnswer | CallState::Busy
        )
    }

    /// Get call duration as a formatted string
    pub fn duration_string(&self) -> String {
        match self.duration_seconds {
            Some(secs) => {
                let mins = secs / 60;
                let remaining_secs = secs % 60;
                format!("{}:{:02}", mins, remaining_secs)
            }
            None => "0:00".to_string(),
        }
    }
}

/// Audio stream direction
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum AudioDirection {
    /// Audio from the remote party (incoming)
    Inbound,
    /// Audio to be sent to the remote party (outgoing)
    Outbound,
}

/// Audio data chunk
#[derive(Debug, Clone)]
pub struct AudioChunk {
    /// The audio data
    pub data: Vec<u8>,

    /// Direction of the audio
    pub direction: AudioDirection,

    /// Sample rate (e.g., 8000, 16000)
    pub sample_rate: u32,

    /// Timestamp
    pub timestamp: DateTime<Utc>,
}

impl AudioChunk {
    pub fn new(data: Vec<u8>, direction: AudioDirection, sample_rate: u32) -> Self {
        AudioChunk {
            data,
            direction,
            sample_rate,
            timestamp: Utc::now(),
        }
    }
}
