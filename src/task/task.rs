//! Task definition and state management

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use uuid::Uuid;

/// A task that Karta will execute
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Task {
    /// Unique task identifier
    pub id: Uuid,

    /// Type of task
    pub task_type: TaskType,

    /// Human-readable description
    pub description: String,

    /// Target to contact (phone number, business name, etc.)
    pub target: TaskTarget,

    /// Current state of the task
    pub state: TaskState,

    /// Task-specific context and parameters
    pub context: TaskContext,

    /// Boundaries specific to this task (override principal defaults)
    #[serde(default)]
    pub boundaries: TaskBoundaries,

    /// History of events during task execution
    #[serde(default)]
    pub history: Vec<TaskEvent>,

    /// When the task was created
    pub created_at: DateTime<Utc>,

    /// When the task was last updated
    pub updated_at: DateTime<Utc>,

    /// When the task was completed (if applicable)
    pub completed_at: Option<DateTime<Utc>>,
}

/// Types of tasks Karta can handle
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "kebab-case")]
pub enum TaskType {
    /// Book an appointment
    BookAppointment,
    /// Complete a rental agreement
    RentalAgreement,
    /// Make a reservation
    Reservation,
    /// Negotiate a bill or dispute
    Negotiation,
    /// General inquiry
    Inquiry,
    /// Follow-up on a previous task
    FollowUp,
    /// Custom task type
    Custom(String),
}

/// The target of a task
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TaskTarget {
    /// Name of the business/person
    pub name: String,

    /// Phone number to call
    pub phone: Option<String>,

    /// Additional context about the target
    #[serde(default)]
    pub context: Option<String>,
}

/// Current state of the task
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum TaskState {
    /// Task is created but not started
    Pending,
    /// Gathering information before the call
    Preparing,
    /// Currently executing (on a call)
    InProgress,
    /// Waiting for principal input
    WaitingForInput,
    /// Call ended, processing results
    Processing,
    /// Task completed successfully
    Completed,
    /// Task failed
    Failed(String),
    /// Task was cancelled
    Cancelled,
}

/// Task-specific context and parameters
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct TaskContext {
    /// The goal - what does success look like?
    pub goal: Option<String>,

    /// Information relevant to this task
    #[serde(default)]
    pub info: HashMap<String, String>,

    /// What's negotiable
    #[serde(default)]
    pub flexible: Vec<String>,

    /// What's fixed/non-negotiable
    #[serde(default)]
    pub firm: Vec<String>,

    /// Notes about the relationship
    pub relationship_context: Option<String>,

    /// Reference to a previous task (for follow-ups)
    pub previous_task_id: Option<Uuid>,
}

/// Task-specific boundaries (override principal defaults)
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct TaskBoundaries {
    /// Budget ceiling for this specific task
    pub budget_ceiling: Option<f64>,

    /// Budget floor (minimum acceptable)
    pub budget_floor: Option<f64>,

    /// Deadline for the task
    pub deadline: Option<DateTime<Utc>>,

    /// Custom constraints
    #[serde(default)]
    pub constraints: Vec<String>,
}

/// An event that occurred during task execution
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TaskEvent {
    /// When the event occurred
    pub timestamp: DateTime<Utc>,

    /// Type of event
    pub event_type: TaskEventType,

    /// Human-readable description
    pub description: String,

    /// Additional data
    #[serde(default)]
    pub data: HashMap<String, String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum TaskEventType {
    /// Task state changed
    StateChange,
    /// Call was made
    CallStarted,
    /// Call ended
    CallEnded,
    /// Something was said
    Utterance,
    /// Agent made a decision
    Decision,
    /// Escalated to principal
    Escalation,
    /// Principal provided input
    PrincipalInput,
    /// An error occurred
    Error,
    /// A note or observation
    Note,
}

impl Task {
    /// Create a new task
    pub fn new(task_type: TaskType, description: String, target: TaskTarget) -> Self {
        let now = Utc::now();
        Task {
            id: Uuid::new_v4(),
            task_type,
            description,
            target,
            state: TaskState::Pending,
            context: TaskContext::default(),
            boundaries: TaskBoundaries::default(),
            history: Vec::new(),
            created_at: now,
            updated_at: now,
            completed_at: None,
        }
    }

    /// Create a task with builder pattern
    pub fn builder() -> TaskBuilder {
        TaskBuilder::new()
    }

    /// Update the task state
    pub fn set_state(&mut self, new_state: TaskState) {
        let old_state = std::mem::replace(&mut self.state, new_state.clone());
        self.updated_at = Utc::now();

        self.add_event(
            TaskEventType::StateChange,
            format!("State changed from {:?} to {:?}", old_state, new_state),
        );

        if matches!(new_state, TaskState::Completed | TaskState::Failed(_) | TaskState::Cancelled) {
            self.completed_at = Some(Utc::now());
        }
    }

    /// Add an event to the history
    pub fn add_event(&mut self, event_type: TaskEventType, description: String) {
        self.history.push(TaskEvent {
            timestamp: Utc::now(),
            event_type,
            description,
            data: HashMap::new(),
        });
        self.updated_at = Utc::now();
    }

    /// Add an event with additional data
    pub fn add_event_with_data(
        &mut self,
        event_type: TaskEventType,
        description: String,
        data: HashMap<String, String>,
    ) {
        self.history.push(TaskEvent {
            timestamp: Utc::now(),
            event_type,
            description,
            data,
        });
        self.updated_at = Utc::now();
    }

    /// Check if the task is complete
    pub fn is_complete(&self) -> bool {
        matches!(
            self.state,
            TaskState::Completed | TaskState::Failed(_) | TaskState::Cancelled
        )
    }

    /// Check if the task is active
    pub fn is_active(&self) -> bool {
        matches!(
            self.state,
            TaskState::Preparing | TaskState::InProgress | TaskState::WaitingForInput | TaskState::Processing
        )
    }

    /// Get the effective budget ceiling (task-specific or None)
    pub fn budget_ceiling(&self) -> Option<f64> {
        self.boundaries.budget_ceiling
    }

    /// Get a summary of the task
    pub fn summary(&self) -> String {
        format!(
            "[{}] {} - {} ({})",
            self.task_type_str(),
            self.target.name,
            self.description,
            self.state_str()
        )
    }

    fn task_type_str(&self) -> &str {
        match &self.task_type {
            TaskType::BookAppointment => "Appointment",
            TaskType::RentalAgreement => "Rental",
            TaskType::Reservation => "Reservation",
            TaskType::Negotiation => "Negotiation",
            TaskType::Inquiry => "Inquiry",
            TaskType::FollowUp => "Follow-up",
            TaskType::Custom(s) => s,
        }
    }

    fn state_str(&self) -> &str {
        match &self.state {
            TaskState::Pending => "Pending",
            TaskState::Preparing => "Preparing",
            TaskState::InProgress => "In Progress",
            TaskState::WaitingForInput => "Waiting for Input",
            TaskState::Processing => "Processing",
            TaskState::Completed => "Completed",
            TaskState::Failed(_) => "Failed",
            TaskState::Cancelled => "Cancelled",
        }
    }
}

/// Builder for creating tasks
pub struct TaskBuilder {
    task_type: Option<TaskType>,
    description: Option<String>,
    target: Option<TaskTarget>,
    context: TaskContext,
    boundaries: TaskBoundaries,
}

impl TaskBuilder {
    pub fn new() -> Self {
        TaskBuilder {
            task_type: None,
            description: None,
            target: None,
            context: TaskContext::default(),
            boundaries: TaskBoundaries::default(),
        }
    }

    pub fn task_type(mut self, task_type: TaskType) -> Self {
        self.task_type = Some(task_type);
        self
    }

    pub fn description(mut self, description: impl Into<String>) -> Self {
        self.description = Some(description.into());
        self
    }

    pub fn target(mut self, name: impl Into<String>, phone: Option<String>) -> Self {
        self.target = Some(TaskTarget {
            name: name.into(),
            phone,
            context: None,
        });
        self
    }

    pub fn target_with_context(
        mut self,
        name: impl Into<String>,
        phone: Option<String>,
        context: impl Into<String>,
    ) -> Self {
        self.target = Some(TaskTarget {
            name: name.into(),
            phone,
            context: Some(context.into()),
        });
        self
    }

    pub fn goal(mut self, goal: impl Into<String>) -> Self {
        self.context.goal = Some(goal.into());
        self
    }

    pub fn info(mut self, key: impl Into<String>, value: impl Into<String>) -> Self {
        self.context.info.insert(key.into(), value.into());
        self
    }

    pub fn flexible(mut self, items: Vec<String>) -> Self {
        self.context.flexible = items;
        self
    }

    pub fn firm(mut self, items: Vec<String>) -> Self {
        self.context.firm = items;
        self
    }

    pub fn budget_ceiling(mut self, ceiling: f64) -> Self {
        self.boundaries.budget_ceiling = Some(ceiling);
        self
    }

    pub fn budget_floor(mut self, floor: f64) -> Self {
        self.boundaries.budget_floor = Some(floor);
        self
    }

    pub fn build(self) -> Result<Task, String> {
        let task_type = self.task_type.ok_or("Task type is required")?;
        let description = self.description.ok_or("Description is required")?;
        let target = self.target.ok_or("Target is required")?;

        let mut task = Task::new(task_type, description, target);
        task.context = self.context;
        task.boundaries = self.boundaries;

        Ok(task)
    }
}

impl Default for TaskBuilder {
    fn default() -> Self {
        Self::new()
    }
}
