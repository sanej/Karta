//! CLI interface for Karta

use clap::{Parser, Subcommand};
use std::path::PathBuf;

/// Karta - Voice-Native AI Executive Assistant
#[derive(Parser, Debug)]
#[command(name = "karta")]
#[command(author, version, about, long_about = None)]
pub struct Cli {
    /// Path to config file
    #[arg(short, long, global = true)]
    pub config: Option<PathBuf>,

    /// Enable verbose output
    #[arg(short, long, global = true)]
    pub verbose: bool,

    /// Run in mock mode (no real calls)
    #[arg(long, global = true)]
    pub mock: bool,

    #[command(subcommand)]
    pub command: Commands,
}

#[derive(Subcommand, Debug)]
pub enum Commands {
    /// Make a call to complete a task
    Call(CallArgs),

    /// Quick test call - just speaks a message
    Test(TestCallArgs),

    /// Manage tasks
    Task(TaskArgs),

    /// Initialize or manage configuration
    Config(ConfigArgs),

    /// Demo mode - run a simulated conversation
    Demo(DemoArgs),
}

/// Arguments for test call
#[derive(Parser, Debug)]
pub struct TestCallArgs {
    /// Phone number to call
    pub phone: String,

    /// Message to speak
    #[arg(short, long, default_value = "Hello! This is Karta, your AI assistant. I'm calling to confirm that the system is working correctly.")]
    pub message: String,
}

/// Arguments for the call command
#[derive(Parser, Debug)]
pub struct CallArgs {
    /// Target to call (business name or phone number)
    pub target: String,

    /// Type of task
    #[arg(short, long, default_value = "inquiry")]
    pub task: TaskTypeArg,

    /// Phone number (if not included in target)
    #[arg(short, long)]
    pub phone: Option<String>,

    /// Goal description
    #[arg(short, long)]
    pub goal: Option<String>,

    /// Budget ceiling for negotiations
    #[arg(long)]
    pub budget_ceiling: Option<f64>,

    /// Items that are flexible/negotiable
    #[arg(long, value_delimiter = ',')]
    pub flexible: Vec<String>,

    /// Items that are firm/non-negotiable
    #[arg(long, value_delimiter = ',')]
    pub firm: Vec<String>,

    /// Additional context
    #[arg(long)]
    pub context: Option<String>,
}

#[derive(Debug, Clone, clap::ValueEnum)]
pub enum TaskTypeArg {
    /// Book an appointment
    Appointment,
    /// Complete a rental agreement
    Rental,
    /// Make a reservation
    Reservation,
    /// Negotiate a bill or dispute
    Negotiation,
    /// General inquiry
    Inquiry,
    /// Follow up on a previous task
    FollowUp,
}

impl From<TaskTypeArg> for crate::task::TaskType {
    fn from(arg: TaskTypeArg) -> Self {
        match arg {
            TaskTypeArg::Appointment => crate::task::TaskType::BookAppointment,
            TaskTypeArg::Rental => crate::task::TaskType::RentalAgreement,
            TaskTypeArg::Reservation => crate::task::TaskType::Reservation,
            TaskTypeArg::Negotiation => crate::task::TaskType::Negotiation,
            TaskTypeArg::Inquiry => crate::task::TaskType::Inquiry,
            TaskTypeArg::FollowUp => crate::task::TaskType::FollowUp,
        }
    }
}

/// Arguments for task management
#[derive(Parser, Debug)]
pub struct TaskArgs {
    #[command(subcommand)]
    pub command: TaskCommands,
}

#[derive(Subcommand, Debug)]
pub enum TaskCommands {
    /// List all tasks
    List {
        /// Include completed tasks
        #[arg(short, long)]
        all: bool,

        /// Limit number of tasks shown
        #[arg(short, long, default_value = "10")]
        limit: usize,
    },

    /// Show details of a specific task
    Show {
        /// Task ID (partial match supported)
        id: String,
    },

    /// Search tasks
    Search {
        /// Search query
        query: String,
    },
}

/// Arguments for configuration
#[derive(Parser, Debug)]
pub struct ConfigArgs {
    #[command(subcommand)]
    pub command: ConfigCommands,
}

#[derive(Subcommand, Debug)]
pub enum ConfigCommands {
    /// Initialize a new configuration
    Init {
        /// Force overwrite existing config
        #[arg(short, long)]
        force: bool,
    },

    /// Show current configuration
    Show,

    /// Edit configuration (opens in editor)
    Edit,

    /// Validate configuration
    Validate,

    /// Show configuration path
    Path,
}

/// Arguments for demo mode
#[derive(Parser, Debug)]
pub struct DemoArgs {
    /// Demo scenario to run
    #[arg(default_value = "appointment")]
    pub scenario: DemoScenario,
}

#[derive(Debug, Clone, clap::ValueEnum)]
pub enum DemoScenario {
    /// Demo appointment booking
    Appointment,
    /// Demo rental negotiation
    Rental,
}

impl Cli {
    pub fn parse_args() -> Self {
        Cli::parse()
    }
}
