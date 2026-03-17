//! Telephony provider trait and types

use async_channel::{Receiver, Sender};
use async_trait::async_trait;

use crate::error::Result;
use crate::telephony::{AudioChunk, Call};

/// Trait for telephony providers (Twilio, mock, etc.)
#[async_trait]
pub trait TelephonyProvider: Send + Sync {
    /// Get the provider name
    fn name(&self) -> &'static str;

    /// Make an outbound call
    async fn make_call(&self, to_number: &str) -> Result<Call>;

    /// End an active call
    async fn end_call(&self, call: &Call) -> Result<()>;

    /// Get the audio stream channels for a call
    /// Returns (inbound_audio_receiver, outbound_audio_sender)
    fn audio_channels(&self, call: &Call) -> Result<(Receiver<AudioChunk>, Sender<AudioChunk>)>;

    /// Send audio to the call
    async fn send_audio(&self, call: &Call, audio: AudioChunk) -> Result<()>;

    /// Check if the provider is connected/ready
    fn is_ready(&self) -> bool;

    /// Get the phone number this provider calls from
    fn from_number(&self) -> &str;
}

/// Events from the telephony provider
#[derive(Debug, Clone)]
pub enum TelephonyEvent {
    /// Call state changed
    CallStateChanged {
        call_id: uuid::Uuid,
        new_state: super::CallState,
    },
    /// Audio received from the remote party
    AudioReceived {
        call_id: uuid::Uuid,
        audio: AudioChunk,
    },
    /// Call connected
    CallConnected { call_id: uuid::Uuid },
    /// Call ended
    CallEnded {
        call_id: uuid::Uuid,
        reason: String,
    },
    /// Error occurred
    Error {
        call_id: Option<uuid::Uuid>,
        error: String,
    },
}

// We need async_trait for the provider trait
// Add this to Cargo.toml:
// async-trait = "0.1"
