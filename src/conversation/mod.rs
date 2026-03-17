//! Conversation module - orchestrates the flow of a call
//!
//! Manages the state machine of a conversation, coordinates between
//! telephony, voice engine, and the terminal UI.

mod state;
mod session;

pub use state::*;
pub use session::*;
