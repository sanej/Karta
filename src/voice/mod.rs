//! Voice engine module - speech-to-text and text-to-speech with LLM
//!
//! Abstracts the voice/conversation AI provider (Gemini Live, OpenAI, etc.)

mod engine;
mod gemini;
mod mock;

pub use engine::*;
pub use gemini::*;
pub use mock::*;

use crate::config::VoiceConfig;
use crate::error::Result;

/// Create a voice engine based on configuration
pub fn create_engine(config: &VoiceConfig) -> Result<Box<dyn VoiceEngine>> {
    match config.provider {
        crate::config::VoiceProvider::Mock => {
            Ok(Box::new(MockVoiceEngine::new()))
        }
        crate::config::VoiceProvider::Gemini => {
            let api_key = config.gemini_api_key.clone()
                .ok_or_else(|| crate::error::KartaError::Config("Gemini API key required".into()))?;
            let model = config.model.clone().unwrap_or_else(|| "gemini-2.0-flash-exp".into());
            Ok(Box::new(GeminiLiveEngine::new(api_key, model)))
        }
        crate::config::VoiceProvider::OpenAI => {
            // OpenAI Realtime API - similar structure to Gemini
            let api_key = config.openai_api_key.clone()
                .ok_or_else(|| crate::error::KartaError::Config("OpenAI API key required".into()))?;
            // For now, fallback to mock with a note
            tracing::warn!("OpenAI Realtime API not yet implemented, using mock");
            Ok(Box::new(MockVoiceEngine::new()))
        }
    }
}
