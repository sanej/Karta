//! Error types for Karta

use thiserror::Error;

#[derive(Error, Debug)]
pub enum KartaError {
    #[error("Configuration error: {0}")]
    Config(String),

    #[error("Principal profile not found at: {0}")]
    ProfileNotFound(String),

    #[error("Task error: {0}")]
    Task(String),

    #[error("Telephony error: {0}")]
    Telephony(String),

    #[error("Voice engine error: {0}")]
    VoiceEngine(String),

    #[error("Connection failed: {0}")]
    Connection(String),

    #[error("API error: {0}")]
    Api(String),

    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    #[error("Serialization error: {0}")]
    Serialization(#[from] serde_json::Error),

    #[error("TOML parse error: {0}")]
    TomlParse(#[from] toml::de::Error),

    #[error("HTTP error: {0}")]
    Http(#[from] reqwest::Error),

    #[error("User cancelled operation")]
    Cancelled,

    #[error("Escalation required: {0}")]
    EscalationRequired(String),
}

pub type Result<T> = std::result::Result<T, KartaError>;
