//! Configuration module for Karta
//!
//! Handles loading and managing the principal profile and agent configuration.

mod principal;
mod agent;

pub use principal::*;
pub use agent::*;

use crate::error::{KartaError, Result};
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

/// Complete Karta configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct KartaConfig {
    pub principal: PrincipalProfile,
    pub agent: AgentConfig,

    #[serde(default)]
    pub telephony: TelephonyConfig,

    #[serde(default)]
    pub voice: VoiceConfig,
}

/// Telephony provider configuration
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct TelephonyConfig {
    pub provider: TelephonyProvider,

    // Twilio
    pub twilio_account_sid: Option<String>,
    pub twilio_auth_token: Option<String>,
    pub twilio_phone_number: Option<String>,

    // Telnyx (alternative - often cheaper)
    pub telnyx_api_key: Option<String>,
    pub telnyx_phone_number: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum TelephonyProvider {
    #[default]
    Mock,
    Twilio,
    Telnyx,
}

/// Voice engine configuration
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct VoiceConfig {
    pub provider: VoiceProvider,
    pub gemini_api_key: Option<String>,
    pub openai_api_key: Option<String>,
    pub model: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum VoiceProvider {
    #[default]
    Mock,
    Gemini,
    OpenAI,
}

impl KartaConfig {
    /// Load configuration from a TOML file
    pub fn load(path: &PathBuf) -> Result<Self> {
        if !path.exists() {
            return Err(KartaError::ProfileNotFound(path.display().to_string()));
        }

        let content = std::fs::read_to_string(path)?;
        let mut config: KartaConfig = toml::from_str(&content)?;

        // Merge with environment variables
        config.merge_env();

        Ok(config)
    }

    /// Load from default location (~/.config/karta/config.toml)
    pub fn load_default() -> Result<Self> {
        let config_path = Self::default_config_path();
        Self::load(&config_path)
    }

    /// Get the default config path
    pub fn default_config_path() -> PathBuf {
        dirs::config_dir()
            .unwrap_or_else(|| PathBuf::from("."))
            .join("karta")
            .join("config.toml")
    }

    /// Save configuration to a file
    pub fn save(&self, path: &PathBuf) -> Result<()> {
        let content = toml::to_string_pretty(self)
            .map_err(|e| KartaError::Config(e.to_string()))?;

        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }

        std::fs::write(path, content)?;
        Ok(())
    }

    /// Create a default configuration
    pub fn default_config() -> Self {
        let mut config = KartaConfig {
            principal: PrincipalProfile::default(),
            agent: AgentConfig::default(),
            telephony: TelephonyConfig::default(),
            voice: VoiceConfig::default(),
        };

        // Merge with environment variables
        config.merge_env();

        config
    }

    /// Merge environment variables into the configuration
    /// Environment variables take precedence over config file values
    pub fn merge_env(&mut self) {
        // Twilio
        if let Ok(val) = std::env::var("TWILIO_ACCOUNT_SID") {
            if !val.is_empty() {
                self.telephony.twilio_account_sid = Some(val);
                self.telephony.provider = TelephonyProvider::Twilio;
            }
        }
        if let Ok(val) = std::env::var("TWILIO_AUTH_TOKEN") {
            if !val.is_empty() {
                self.telephony.twilio_auth_token = Some(val);
            }
        }
        if let Ok(val) = std::env::var("TWILIO_PHONE_NUMBER") {
            if !val.is_empty() {
                self.telephony.twilio_phone_number = Some(val);
            }
        }

        // Telnyx (alternative)
        if let Ok(val) = std::env::var("TELNYX_API_KEY") {
            if !val.is_empty() {
                self.telephony.telnyx_api_key = Some(val);
                self.telephony.provider = TelephonyProvider::Telnyx;
            }
        }
        if let Ok(val) = std::env::var("TELNYX_PHONE_NUMBER") {
            if !val.is_empty() {
                self.telephony.telnyx_phone_number = Some(val);
            }
        }

        // Gemini
        if let Ok(val) = std::env::var("GEMINI_API_KEY") {
            if !val.is_empty() {
                self.voice.gemini_api_key = Some(val);
                self.voice.provider = VoiceProvider::Gemini;
            }
        }

        // OpenAI
        if let Ok(val) = std::env::var("OPENAI_API_KEY") {
            if !val.is_empty() {
                self.voice.openai_api_key = Some(val);
                // Only set OpenAI as provider if Gemini isn't already set
                if self.voice.gemini_api_key.is_none() {
                    self.voice.provider = VoiceProvider::OpenAI;
                }
            }
        }
    }
}
