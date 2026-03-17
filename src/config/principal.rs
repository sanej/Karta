//! Principal Profile - Your values, boundaries, and preferences
//!
//! This is the soul of Karta. It encodes who you are and what boundaries
//! the agent must operate within.

use serde::{Deserialize, Serialize};
use std::collections::HashSet;

/// The principal profile - represents you, the person Karta acts on behalf of
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PrincipalProfile {
    /// Your name
    pub name: String,

    /// Your location (for context)
    #[serde(default)]
    pub location: Option<String>,

    /// Your email
    #[serde(default)]
    pub email: Option<String>,

    /// Your phone number
    #[serde(default)]
    pub phone: Option<String>,

    /// Your core values that guide the agent
    pub values: PrincipalValues,

    /// Hard boundaries the agent must never cross
    pub boundaries: PrincipalBoundaries,

    /// Preferences for how things should be done
    #[serde(default)]
    pub preferences: PrincipalPreferences,
}

/// Your core values that guide agent behavior
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PrincipalValues {
    /// Honesty policy - how transparent should the agent be?
    #[serde(default = "default_honesty")]
    pub honesty: String,

    /// Privacy stance - how protective of your information?
    #[serde(default = "default_privacy")]
    pub privacy_level: PrivacyLevel,

    /// Financial risk tolerance
    #[serde(default)]
    pub financial_risk: RiskLevel,

    /// Time sensitivity - how important is speed vs. thoroughness?
    #[serde(default)]
    pub time_sensitivity: TimeSensitivity,

    /// Custom values you want to encode
    #[serde(default)]
    pub custom: Vec<CustomValue>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CustomValue {
    pub name: String,
    pub description: String,
}

/// Hard boundaries that must never be crossed
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PrincipalBoundaries {
    /// Maximum amount the agent can approve without checking with you
    #[serde(default = "default_max_auto_approve")]
    pub max_auto_approve_amount: f64,

    /// Information that CAN be shared
    #[serde(default)]
    pub share_allowed: HashSet<String>,

    /// Information that must NEVER be shared
    #[serde(default = "default_share_never")]
    pub share_never: HashSet<String>,

    /// Commitments that require your explicit approval
    #[serde(default = "default_never_commit")]
    pub never_commit_without_approval: HashSet<String>,

    /// Actions that should always escalate to you
    #[serde(default)]
    pub always_escalate: HashSet<String>,
}

/// Preferences for how things should be handled
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct PrincipalPreferences {
    /// Preferred times for appointments
    #[serde(default)]
    pub preferred_times: Vec<String>,

    /// Communication style preference
    #[serde(default)]
    pub communication_style: CommunicationStyle,

    /// How long to wait on hold before escalating
    #[serde(default = "default_hold_patience")]
    pub hold_patience_minutes: u32,

    /// Whether to accept voicemail or keep trying
    #[serde(default)]
    pub voicemail_strategy: VoicemailStrategy,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum PrivacyLevel {
    /// Share minimal information, only what's absolutely required
    Conservative,
    #[default]
    /// Share reasonable information for the task at hand
    Moderate,
    /// Share freely to expedite tasks
    Open,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum RiskLevel {
    /// Very careful with financial decisions
    Low,
    #[default]
    /// Reasonable risk tolerance
    Moderate,
    /// Willing to take calculated risks
    High,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum TimeSensitivity {
    /// Take time to get things right
    Relaxed,
    #[default]
    /// Balance speed and thoroughness
    Balanced,
    /// Prioritize speed
    Urgent,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum CommunicationStyle {
    #[default]
    /// Professional and courteous
    Professional,
    /// Warm and friendly
    Friendly,
    /// Direct and efficient
    Direct,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum VoicemailStrategy {
    #[default]
    /// Leave a message and wait
    LeaveMessage,
    /// Keep calling back
    KeepTrying,
    /// Escalate to principal
    Escalate,
}

// Default value functions
fn default_honesty() -> String {
    "Always disclose AI status if directly asked".to_string()
}

fn default_privacy() -> PrivacyLevel {
    PrivacyLevel::Moderate
}

fn default_max_auto_approve() -> f64 {
    100.0
}

fn default_share_never() -> HashSet<String> {
    ["ssn", "bank_account", "credit_card", "password"]
        .iter()
        .map(|s| s.to_string())
        .collect()
}

fn default_never_commit() -> HashSet<String> {
    ["in-person meetings", "financial agreements", "legal documents"]
        .iter()
        .map(|s| s.to_string())
        .collect()
}

fn default_hold_patience() -> u32 {
    15
}

impl Default for PrincipalProfile {
    fn default() -> Self {
        PrincipalProfile {
            name: "User".to_string(),
            location: None,
            email: None,
            phone: None,
            values: PrincipalValues::default(),
            boundaries: PrincipalBoundaries::default(),
            preferences: PrincipalPreferences::default(),
        }
    }
}

impl Default for PrincipalValues {
    fn default() -> Self {
        PrincipalValues {
            honesty: default_honesty(),
            privacy_level: default_privacy(),
            financial_risk: RiskLevel::default(),
            time_sensitivity: TimeSensitivity::default(),
            custom: Vec::new(),
        }
    }
}

impl Default for PrincipalBoundaries {
    fn default() -> Self {
        PrincipalBoundaries {
            max_auto_approve_amount: default_max_auto_approve(),
            share_allowed: ["name", "email", "phone"]
                .iter()
                .map(|s| s.to_string())
                .collect(),
            share_never: default_share_never(),
            never_commit_without_approval: default_never_commit(),
            always_escalate: HashSet::new(),
        }
    }
}

impl PrincipalProfile {
    /// Check if a piece of information can be shared
    pub fn can_share(&self, info_type: &str) -> bool {
        let info_lower = info_type.to_lowercase();

        // Never share if in the never list
        if self.boundaries.share_never.iter().any(|s| s.to_lowercase() == info_lower) {
            return false;
        }

        // Check privacy level for items not explicitly listed
        if self.boundaries.share_allowed.iter().any(|s| s.to_lowercase() == info_lower) {
            return true;
        }

        // Default based on privacy level
        match self.values.privacy_level {
            PrivacyLevel::Conservative => false,
            PrivacyLevel::Moderate => false, // Require explicit allow
            PrivacyLevel::Open => true,
        }
    }

    /// Check if an amount requires approval
    pub fn requires_approval(&self, amount: f64) -> bool {
        amount > self.boundaries.max_auto_approve_amount
    }

    /// Check if a commitment type requires explicit approval
    pub fn commitment_requires_approval(&self, commitment_type: &str) -> bool {
        self.boundaries
            .never_commit_without_approval
            .iter()
            .any(|c| commitment_type.to_lowercase().contains(&c.to_lowercase()))
    }

    /// Check if something should always escalate
    pub fn should_escalate(&self, action: &str) -> bool {
        self.boundaries
            .always_escalate
            .iter()
            .any(|a| action.to_lowercase().contains(&a.to_lowercase()))
    }
}
