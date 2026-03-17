//! Conversation state machine

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Current state of a conversation
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum ConversationState {
    /// Not yet started
    NotStarted,
    /// Initiating the call
    Initiating,
    /// Waiting for the call to connect
    Connecting,
    /// Call is connected, conversation active
    Active,
    /// Waiting for principal input
    WaitingForPrincipal(WaitingContext),
    /// Call is on hold
    OnHold,
    /// Wrapping up the call
    Ending,
    /// Call has ended
    Ended(EndReason),
}

/// Context for when we're waiting for principal input
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct WaitingContext {
    /// The question being asked
    pub question: String,
    /// Suggested options (if any)
    pub options: Vec<String>,
    /// How urgent is the input needed
    pub urgency: Urgency,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum Urgency {
    Low,
    Medium,
    High,
}

/// Reason for ending the conversation
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum EndReason {
    /// Task completed successfully
    Completed,
    /// Task failed
    Failed(String),
    /// Principal requested end
    PrincipalEnded,
    /// Other party ended the call
    RemoteEnded,
    /// Technical error
    Error(String),
    /// Timed out
    Timeout,
}

/// Events that can occur during a conversation
#[derive(Debug, Clone)]
pub enum ConversationEvent {
    /// Call initiated
    CallInitiated,
    /// Call connected
    CallConnected,
    /// Received audio from remote
    AudioReceived(Vec<u8>),
    /// Agent spoke
    AgentSpoke(String),
    /// Remote party spoke
    RemoteSpoke(String),
    /// Need principal input
    NeedInput(WaitingContext),
    /// Principal provided input
    PrincipalInput(String),
    /// Agent decided something
    Decision(String),
    /// Call ending
    CallEnding(EndReason),
    /// Call ended
    CallEnded,
    /// Error occurred
    Error(String),
}

/// Manages conversation state transitions
pub struct ConversationStateMachine {
    state: ConversationState,
    history: Vec<StateTransition>,
}

#[derive(Debug, Clone)]
pub struct StateTransition {
    pub from: ConversationState,
    pub to: ConversationState,
    pub event: String,
    pub timestamp: chrono::DateTime<chrono::Utc>,
}

impl ConversationStateMachine {
    pub fn new() -> Self {
        ConversationStateMachine {
            state: ConversationState::NotStarted,
            history: Vec::new(),
        }
    }

    /// Get current state
    pub fn state(&self) -> &ConversationState {
        &self.state
    }

    /// Process an event and transition state
    pub fn process_event(&mut self, event: ConversationEvent) -> Result<(), String> {
        let new_state = self.next_state(&event)?;

        let transition = StateTransition {
            from: self.state.clone(),
            to: new_state.clone(),
            event: format!("{:?}", event),
            timestamp: chrono::Utc::now(),
        };

        self.history.push(transition);
        self.state = new_state;

        Ok(())
    }

    /// Determine the next state based on current state and event
    fn next_state(&self, event: &ConversationEvent) -> Result<ConversationState, String> {
        match (&self.state, event) {
            // Starting the call
            (ConversationState::NotStarted, ConversationEvent::CallInitiated) => {
                Ok(ConversationState::Initiating)
            }

            (ConversationState::Initiating, ConversationEvent::CallConnected) => {
                Ok(ConversationState::Connecting)
            }

            (ConversationState::Connecting, ConversationEvent::AudioReceived(_)) |
            (ConversationState::Connecting, ConversationEvent::RemoteSpoke(_)) => {
                Ok(ConversationState::Active)
            }

            // Active conversation
            (ConversationState::Active, ConversationEvent::AudioReceived(_)) |
            (ConversationState::Active, ConversationEvent::AgentSpoke(_)) |
            (ConversationState::Active, ConversationEvent::RemoteSpoke(_)) |
            (ConversationState::Active, ConversationEvent::Decision(_)) => {
                Ok(ConversationState::Active)
            }

            (ConversationState::Active, ConversationEvent::NeedInput(ctx)) => {
                Ok(ConversationState::WaitingForPrincipal(ctx.clone()))
            }

            // Waiting for principal
            (ConversationState::WaitingForPrincipal(_), ConversationEvent::PrincipalInput(_)) => {
                Ok(ConversationState::Active)
            }

            // Ending
            (_, ConversationEvent::CallEnding(reason)) => {
                Ok(ConversationState::Ending)
            }

            (ConversationState::Ending, ConversationEvent::CallEnded) => {
                Ok(ConversationState::Ended(EndReason::Completed))
            }

            (_, ConversationEvent::Error(msg)) => {
                Ok(ConversationState::Ended(EndReason::Error(msg.clone())))
            }

            // Invalid transitions
            (state, event) => {
                Err(format!("Invalid transition: {:?} + {:?}", state, event))
            }
        }
    }

    /// Check if the conversation is still active
    pub fn is_active(&self) -> bool {
        !matches!(self.state, ConversationState::NotStarted | ConversationState::Ended(_))
    }

    /// Check if we're waiting for principal input
    pub fn is_waiting_for_input(&self) -> bool {
        matches!(self.state, ConversationState::WaitingForPrincipal(_))
    }

    /// Get the waiting context if waiting for input
    pub fn waiting_context(&self) -> Option<&WaitingContext> {
        match &self.state {
            ConversationState::WaitingForPrincipal(ctx) => Some(ctx),
            _ => None,
        }
    }

    /// Get the conversation history
    pub fn history(&self) -> &[StateTransition] {
        &self.history
    }
}

impl Default for ConversationStateMachine {
    fn default() -> Self {
        Self::new()
    }
}
