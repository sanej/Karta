//! Telephony module - making and managing phone calls
//!
//! Abstracts the telephony provider (Twilio, Telnyx, mock, etc.)

mod provider;
mod twilio;
mod telnyx;
mod mock;
mod call;

pub use provider::*;
pub use twilio::*;
pub use telnyx::*;
pub use mock::*;
pub use call::*;

use crate::config::TelephonyConfig;
use crate::error::Result;

/// Create a telephony provider based on configuration
pub fn create_provider(config: &TelephonyConfig) -> Result<Box<dyn TelephonyProvider>> {
    match config.provider {
        crate::config::TelephonyProvider::Mock => {
            Ok(Box::new(MockTelephonyProvider::new()))
        }
        crate::config::TelephonyProvider::Twilio => {
            let account_sid = config.twilio_account_sid.clone()
                .ok_or_else(|| crate::error::KartaError::Config("Twilio account SID required. Set TWILIO_ACCOUNT_SID in .env".into()))?;
            let auth_token = config.twilio_auth_token.clone()
                .ok_or_else(|| crate::error::KartaError::Config("Twilio auth token required. Set TWILIO_AUTH_TOKEN in .env".into()))?;
            let phone_number = config.twilio_phone_number.clone()
                .ok_or_else(|| crate::error::KartaError::Config("Twilio phone number required. Set TWILIO_PHONE_NUMBER in .env".into()))?;

            Ok(Box::new(TwilioProvider::new(account_sid, auth_token, phone_number)))
        }
        crate::config::TelephonyProvider::Telnyx => {
            let api_key = config.telnyx_api_key.clone()
                .ok_or_else(|| crate::error::KartaError::Config("Telnyx API key required. Set TELNYX_API_KEY in .env".into()))?;
            let phone_number = config.telnyx_phone_number.clone()
                .ok_or_else(|| crate::error::KartaError::Config("Telnyx phone number required. Set TELNYX_PHONE_NUMBER in .env".into()))?;

            Ok(Box::new(TelnyxProvider::new(api_key, phone_number)))
        }
    }
}
