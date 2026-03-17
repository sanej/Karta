//! Telnyx telephony provider
//!
//! Integrates with Telnyx for real phone calls.
//! Often 25-45% cheaper than Twilio for US voice/SMS.

use async_channel::{bounded, Receiver, Sender};
use async_trait::async_trait;
use std::sync::Arc;
use tokio::sync::Mutex;

use crate::error::{KartaError, Result};
use crate::telephony::{AudioChunk, Call, CallState, TelephonyProvider};

/// Telnyx telephony provider
pub struct TelnyxProvider {
    api_key: String,
    from_number: String,
    client: reqwest::Client,
    active_calls: Arc<Mutex<Vec<TelnyxCall>>>,
}

struct TelnyxCall {
    call: Call,
    _inbound_tx: Sender<AudioChunk>,
    _outbound_rx: Receiver<AudioChunk>,
}

impl TelnyxProvider {
    pub fn new(api_key: String, from_number: String) -> Self {
        TelnyxProvider {
            api_key,
            from_number,
            client: reqwest::Client::new(),
            active_calls: Arc::new(Mutex::new(Vec::new())),
        }
    }

    /// Make a Telnyx API call to initiate a phone call
    async fn create_telnyx_call(&self, to_number: &str) -> Result<String> {
        let url = "https://api.telnyx.com/v2/calls";

        let body = serde_json::json!({
            "connection_id": "your_connection_id", // Would come from config
            "to": to_number,
            "from": self.from_number,
            "webhook_url": "https://your-server.com/telnyx-webhook",
            "stream_url": "wss://your-server.com/telnyx-stream"
        });

        let response = self
            .client
            .post(url)
            .bearer_auth(&self.api_key)
            .json(&body)
            .send()
            .await?;

        if !response.status().is_success() {
            let error_text = response.text().await.unwrap_or_default();
            return Err(KartaError::Telephony(format!(
                "Telnyx API error: {}",
                error_text
            )));
        }

        let json: serde_json::Value = response.json().await?;
        let call_control_id = json["data"]["call_control_id"]
            .as_str()
            .ok_or_else(|| KartaError::Telephony("Missing call_control_id in response".into()))?
            .to_string();

        Ok(call_control_id)
    }
}

#[async_trait]
impl TelephonyProvider for TelnyxProvider {
    fn name(&self) -> &'static str {
        "telnyx"
    }

    async fn make_call(&self, to_number: &str) -> Result<Call> {
        let mut call = Call::new(to_number.to_string(), self.from_number.clone());
        call.state = CallState::Initiating;

        // Make the Telnyx API call
        let call_control_id = self.create_telnyx_call(to_number).await?;
        call.provider_call_id = Some(call_control_id);
        call.state = CallState::Ringing;

        // Create audio channels
        let (inbound_tx, _inbound_rx) = bounded::<AudioChunk>(100);
        let (_outbound_tx, outbound_rx) = bounded::<AudioChunk>(100);

        let telnyx_call = TelnyxCall {
            call: call.clone(),
            _inbound_tx: inbound_tx,
            _outbound_rx: outbound_rx,
        };

        let mut calls = self.active_calls.lock().await;
        calls.push(telnyx_call);

        Ok(call)
    }

    async fn end_call(&self, call: &Call) -> Result<()> {
        if let Some(ref call_control_id) = call.provider_call_id {
            let url = format!(
                "https://api.telnyx.com/v2/calls/{}/actions/hangup",
                call_control_id
            );

            self.client
                .post(&url)
                .bearer_auth(&self.api_key)
                .json(&serde_json::json!({}))
                .send()
                .await?;
        }

        let mut calls = self.active_calls.lock().await;
        if let Some(c) = calls.iter_mut().find(|c| c.call.id == call.id) {
            c.call.end();
        }

        Ok(())
    }

    fn audio_channels(&self, _call: &Call) -> Result<(Receiver<AudioChunk>, Sender<AudioChunk>)> {
        let (inbound_tx, inbound_rx) = bounded::<AudioChunk>(100);
        let (outbound_tx, _outbound_rx) = bounded::<AudioChunk>(100);

        // In a full implementation, these would connect to Telnyx's
        // WebSocket streaming API

        Ok((inbound_rx, outbound_tx))
    }

    async fn send_audio(&self, call: &Call, _audio: AudioChunk) -> Result<()> {
        let calls = self.active_calls.lock().await;
        if let Some(_c) = calls.iter().find(|c| c.call.id == call.id) {
            // Send audio through WebSocket
        }
        Ok(())
    }

    fn is_ready(&self) -> bool {
        !self.api_key.is_empty()
    }

    fn from_number(&self) -> &str {
        &self.from_number
    }
}
