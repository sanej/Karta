//! Conversation session - orchestrates a complete call

use async_channel::{bounded, Receiver, Sender};
use std::sync::Arc;
use tokio::sync::Mutex;

use crate::config::{AgentConfig, KartaConfig, PrincipalProfile};
use crate::conversation::{ConversationEvent, ConversationState, ConversationStateMachine, WaitingContext, Urgency};
use crate::error::{KartaError, Result};
use crate::task::{Task, TaskEventType, TaskState};
use crate::telephony::{Call, TelephonyProvider};
use crate::voice::{TranscriptEvent, VoiceEngine, Speaker};

/// A conversation session manages a single call from start to finish
pub struct ConversationSession {
    /// The task being executed
    task: Task,

    /// Configuration
    config: KartaConfig,

    /// State machine
    state_machine: ConversationStateMachine,

    /// Current call (if active)
    call: Option<Call>,

    /// Channel for UI events
    ui_tx: Sender<UIEvent>,
    ui_rx: Receiver<UIEvent>,

    /// Channel for principal input
    input_tx: Sender<String>,
    input_rx: Receiver<String>,
}

/// Events sent to the UI
#[derive(Debug, Clone)]
pub enum UIEvent {
    /// Status update
    Status(String),
    /// Transcript update
    Transcript(TranscriptEvent),
    /// Need input from principal
    NeedInput(WaitingContext),
    /// Call state changed
    CallStateChanged(ConversationState),
    /// Task completed
    TaskCompleted(bool, String),
    /// Error
    Error(String),
}

impl ConversationSession {
    /// Create a new conversation session
    pub fn new(task: Task, config: KartaConfig) -> Self {
        let (ui_tx, ui_rx) = bounded(100);
        let (input_tx, input_rx) = bounded(10);

        ConversationSession {
            task,
            config,
            state_machine: ConversationStateMachine::new(),
            call: None,
            ui_tx,
            ui_rx,
            input_tx,
            input_rx,
        }
    }

    /// Get the UI event receiver
    pub fn ui_events(&self) -> Receiver<UIEvent> {
        self.ui_rx.clone()
    }

    /// Get the input sender for principal to provide input
    pub fn input_sender(&self) -> Sender<String> {
        self.input_tx.clone()
    }

    /// Get the current task
    pub fn task(&self) -> &Task {
        &self.task
    }

    /// Get the current state
    pub fn state(&self) -> &ConversationState {
        self.state_machine.state()
    }

    /// Run the conversation session
    pub async fn run(
        &mut self,
        telephony: Box<dyn TelephonyProvider>,
        mut voice_engine: Box<dyn VoiceEngine>,
    ) -> Result<()> {
        // Generate the system prompt
        let system_prompt = self.generate_system_prompt();

        // Send status
        self.send_ui_event(UIEvent::Status("Preparing call...".into())).await;

        // Initialize the voice engine
        voice_engine.start_session(&system_prompt).await?;

        // Get transcript channel
        let transcript_rx = voice_engine.transcript_channel();

        // Start transcript forwarding
        if let Some(rx) = transcript_rx {
            let ui_tx = self.ui_tx.clone();
            tokio::spawn(async move {
                while let Ok(event) = rx.recv().await {
                    let _ = ui_tx.send(UIEvent::Transcript(event)).await;
                }
            });
        }

        // Get the phone number to call
        let phone_number = self.task.target.phone.clone()
            .ok_or_else(|| KartaError::Task("No phone number for target".into()))?;

        // Make the call
        self.send_ui_event(UIEvent::Status(format!("Calling {}...", self.task.target.name))).await;
        self.state_machine.process_event(ConversationEvent::CallInitiated).ok();

        let call = telephony.make_call(&phone_number).await?;
        self.call = Some(call.clone());

        self.task.set_state(TaskState::InProgress);
        self.task.add_event(TaskEventType::CallStarted, format!("Calling {}", phone_number));

        self.state_machine.process_event(ConversationEvent::CallConnected).ok();
        self.send_ui_event(UIEvent::Status("Connected!".into())).await;
        self.send_ui_event(UIEvent::CallStateChanged(self.state().clone())).await;

        // Main conversation loop
        let mut loop_count = 0;
        let max_loops = 100; // Safety limit

        while self.state_machine.is_active() && loop_count < max_loops {
            loop_count += 1;

            // Check for principal input if we're waiting
            if self.state_machine.is_waiting_for_input() {
                if let Ok(input) = self.input_rx.try_recv() {
                    // Process principal input
                    self.state_machine.process_event(ConversationEvent::PrincipalInput(input.clone())).ok();

                    // Send to voice engine
                    let response = voice_engine.process_text(&input).await?;

                    // Handle response
                    self.handle_voice_response(&response).await?;
                }
            }

            // In mock mode, simulate conversation flow
            // In real mode, audio would come from telephony provider
            tokio::time::sleep(tokio::time::Duration::from_millis(500)).await;

            // Process audio (in mock mode this triggers scripted responses)
            let response = voice_engine.process_audio(&[]).await?;

            // Handle the response
            if self.handle_voice_response(&response).await? {
                // Response indicated we should end
                break;
            }
        }

        // End the call
        if let Some(ref call) = self.call {
            telephony.end_call(call).await?;
        }

        self.state_machine.process_event(ConversationEvent::CallEnded).ok();
        voice_engine.end_session().await?;

        // Update task state
        let success = matches!(self.state(), ConversationState::Ended(crate::conversation::EndReason::Completed));
        self.task.set_state(if success {
            TaskState::Completed
        } else {
            TaskState::Failed("Call ended without completing task".into())
        });

        self.task.add_event(TaskEventType::CallEnded, "Call ended".to_string());

        self.send_ui_event(UIEvent::TaskCompleted(
            success,
            if success { "Task completed successfully" } else { "Task incomplete" }.into()
        )).await;

        Ok(())
    }

    /// Handle a response from the voice engine
    async fn handle_voice_response(&mut self, response: &crate::voice::VoiceResponse) -> Result<bool> {
        // Handle transcript
        if let Some(ref text) = response.transcript_in {
            self.state_machine.process_event(ConversationEvent::RemoteSpoke(text.clone())).ok();
        }

        if let Some(ref text) = response.response_text {
            self.state_machine.process_event(ConversationEvent::AgentSpoke(text.clone())).ok();
        }

        // Handle need for input
        if response.needs_input {
            let ctx = WaitingContext {
                question: response.input_prompt.clone().unwrap_or_default(),
                options: Vec::new(),
                urgency: Urgency::Medium,
            };

            self.state_machine.process_event(ConversationEvent::NeedInput(ctx.clone())).ok();
            self.send_ui_event(UIEvent::NeedInput(ctx)).await;
            self.send_ui_event(UIEvent::CallStateChanged(self.state().clone())).await;
        }

        // Handle end
        if response.should_end {
            let reason = response.end_reason.clone().unwrap_or_else(|| "completed".into());
            self.task.add_event(TaskEventType::Note, format!("Ending: {}", reason));

            self.state_machine.process_event(
                ConversationEvent::CallEnding(crate::conversation::EndReason::Completed)
            ).ok();

            return Ok(true);
        }

        Ok(false)
    }

    /// Generate the system prompt for the voice engine
    fn generate_system_prompt(&self) -> String {
        let agent_prompt = self.config.agent.to_system_prompt();
        let principal = &self.config.principal;
        let task = &self.task;

        let mut prompt = agent_prompt;

        // Add principal context
        prompt.push_str(&format!(r#"

PRINCIPAL INFORMATION:
- Name: {}
- Location: {}
- Privacy Level: {:?}

PRINCIPAL BOUNDARIES:
- Max auto-approve amount: ${}
- Can share: {}
- Never share: {}
- Requires approval: {}
"#,
            principal.name,
            principal.location.as_deref().unwrap_or("Not specified"),
            principal.values.privacy_level,
            principal.boundaries.max_auto_approve_amount,
            principal.boundaries.share_allowed.iter().cloned().collect::<Vec<_>>().join(", "),
            principal.boundaries.share_never.iter().cloned().collect::<Vec<_>>().join(", "),
            principal.boundaries.never_commit_without_approval.iter().cloned().collect::<Vec<_>>().join(", "),
        ));

        // Add task context
        prompt.push_str(&format!(r#"

CURRENT TASK:
- Type: {:?}
- Target: {}
- Description: {}
- Goal: {}
"#,
            task.task_type,
            task.target.name,
            task.description,
            task.context.goal.as_deref().unwrap_or("Complete the task"),
        ));

        // Add task-specific constraints
        if let Some(ceiling) = task.boundaries.budget_ceiling {
            prompt.push_str(&format!("- Budget ceiling: ${}\n", ceiling));
        }
        if !task.context.flexible.is_empty() {
            prompt.push_str(&format!("- Flexible on: {}\n", task.context.flexible.join(", ")));
        }
        if !task.context.firm.is_empty() {
            prompt.push_str(&format!("- Firm on: {}\n", task.context.firm.join(", ")));
        }

        prompt.push_str(r#"

INSTRUCTIONS:
1. Make the call and accomplish the task
2. Stay within the boundaries set by the principal
3. If you need to make a decision outside your authority, use request_principal_input
4. Be efficient but polite
5. When the task is complete, use end_call with success=true
"#);

        prompt
    }

    async fn send_ui_event(&self, event: UIEvent) {
        let _ = self.ui_tx.send(event).await;
    }
}
