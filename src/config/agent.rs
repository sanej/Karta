//! Agent Configuration - The agent's personality and operating style
//!
//! This defines how Karta shows up in conversations - its tone,
//! negotiation style, and behavioral patterns.

use serde::{Deserialize, Serialize};

/// Agent configuration - defines Karta's personality
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentConfig {
    /// The agent's name (what it calls itself)
    #[serde(default = "default_agent_name")]
    pub name: String,

    /// Agent's personality traits
    pub personality: AgentPersonality,

    /// Agent's core values (how it behaves)
    pub values: AgentValues,

    /// Operational parameters
    #[serde(default)]
    pub operations: AgentOperations,
}

/// The agent's personality - how it shows up in conversations
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentPersonality {
    /// Overall tone of communication
    #[serde(default)]
    pub tone: Tone,

    /// Negotiation approach
    #[serde(default)]
    pub negotiation_style: NegotiationStyle,

    /// How patient with delays and obstacles
    #[serde(default)]
    pub patience: PatienceLevel,

    /// How assertive in pursuing goals
    #[serde(default)]
    pub assertiveness: AssertivenessLevel,

    /// How much detail in communications
    #[serde(default)]
    pub verbosity: Verbosity,
}

/// Agent's operating values - its principles
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentValues {
    /// Respect for others on the call
    #[serde(default = "default_respect")]
    pub respect: String,

    /// Persistence in achieving goals
    #[serde(default = "default_persistence")]
    pub persistence: String,

    /// Transparency about being an AI assistant
    #[serde(default = "default_transparency")]
    pub transparency: String,

    /// Additional custom values
    #[serde(default)]
    pub custom: Vec<AgentCustomValue>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentCustomValue {
    pub name: String,
    pub principle: String,
}

/// Operational parameters
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct AgentOperations {
    /// Maximum call duration in minutes before escalating
    #[serde(default = "default_max_call_duration")]
    pub max_call_duration_minutes: u32,

    /// How many times to retry a failed call
    #[serde(default = "default_retry_attempts")]
    pub retry_attempts: u32,

    /// Seconds to wait between retries
    #[serde(default = "default_retry_delay")]
    pub retry_delay_seconds: u32,

    /// Whether to record calls (for review)
    #[serde(default)]
    pub record_calls: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default, PartialEq)]
#[serde(rename_all = "kebab-case")]
pub enum Tone {
    /// Professional and businesslike
    Professional,
    #[default]
    /// Warm but professional
    WarmProfessional,
    /// Friendly and casual
    Friendly,
    /// Very direct, no small talk
    Direct,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default, PartialEq)]
#[serde(rename_all = "kebab-case")]
pub enum NegotiationStyle {
    /// Work together to find solutions
    #[default]
    Collaborative,
    /// Collaborative but firm on key points
    CollaborativeFirm,
    /// Start with strong position, negotiate down
    Anchoring,
    /// Accept reasonable offers quickly
    Accommodating,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum PatienceLevel {
    /// Will wait and be very understanding
    High,
    #[default]
    /// Reasonable patience
    Medium,
    /// Prefers quick resolution
    Low,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum AssertivenessLevel {
    /// Very gentle, avoids pushback
    Low,
    #[default]
    /// Balanced assertiveness
    Moderate,
    /// Pushes firmly for outcomes
    High,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum Verbosity {
    /// Very brief responses
    Terse,
    #[default]
    /// Clear but not wordy
    Concise,
    /// Thorough explanations
    Detailed,
}

// Default value functions
fn default_agent_name() -> String {
    "Karta".to_string()
}

fn default_respect() -> String {
    "Always be respectful to the person on the other end".to_string()
}

fn default_persistence() -> String {
    "Don't give up easily but know when to escalate".to_string()
}

fn default_transparency() -> String {
    "Be upfront about acting on someone's behalf".to_string()
}

fn default_max_call_duration() -> u32 {
    30
}

fn default_retry_attempts() -> u32 {
    3
}

fn default_retry_delay() -> u32 {
    60
}

impl Default for AgentConfig {
    fn default() -> Self {
        AgentConfig {
            name: default_agent_name(),
            personality: AgentPersonality::default(),
            values: AgentValues::default(),
            operations: AgentOperations::default(),
        }
    }
}

impl Default for AgentPersonality {
    fn default() -> Self {
        AgentPersonality {
            tone: Tone::default(),
            negotiation_style: NegotiationStyle::default(),
            patience: PatienceLevel::default(),
            assertiveness: AssertivenessLevel::default(),
            verbosity: Verbosity::default(),
        }
    }
}

impl Default for AgentValues {
    fn default() -> Self {
        AgentValues {
            respect: default_respect(),
            persistence: default_persistence(),
            transparency: default_transparency(),
            custom: Vec::new(),
        }
    }
}

impl AgentConfig {
    /// Generate a system prompt that encodes the agent's personality
    pub fn to_system_prompt(&self) -> String {
        let tone_desc = match self.personality.tone {
            Tone::Professional => "professional and businesslike",
            Tone::WarmProfessional => "warm but professional",
            Tone::Friendly => "friendly and approachable",
            Tone::Direct => "direct and efficient, minimal small talk",
        };

        let negotiation_desc = match self.personality.negotiation_style {
            NegotiationStyle::Collaborative => "work collaboratively to find mutually beneficial solutions",
            NegotiationStyle::CollaborativeFirm => "be collaborative but firm on key points",
            NegotiationStyle::Anchoring => "start with a strong position and negotiate from there",
            NegotiationStyle::Accommodating => "be accommodating and accept reasonable offers",
        };

        let assertiveness_desc = match self.personality.assertiveness {
            AssertivenessLevel::Low => "gentle and avoid confrontation",
            AssertivenessLevel::Moderate => "balanced - push when needed but don't be aggressive",
            AssertivenessLevel::High => "assertive in pursuing the goals",
        };

        format!(
            r#"You are {name}, an AI assistant acting on behalf of a principal.

PERSONALITY:
- Tone: {tone_desc}
- Negotiation approach: {negotiation_desc}
- Assertiveness: {assertiveness_desc}

CORE VALUES:
- Respect: {respect}
- Persistence: {persistence}
- Transparency: {transparency}

When on a call:
1. Introduce yourself as {name}, calling on behalf of the principal
2. Be {tone_desc} in all interactions
3. {negotiation_desc}
4. If you need input from the principal, pause and request it
5. Stay within the boundaries set by the principal
"#,
            name = self.name,
            tone_desc = tone_desc,
            negotiation_desc = negotiation_desc,
            assertiveness_desc = assertiveness_desc,
            respect = self.values.respect,
            persistence = self.values.persistence,
            transparency = self.values.transparency,
        )
    }
}
