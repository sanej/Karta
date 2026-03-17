//! Karta - Voice-Native AI Executive Assistant
//!
//! An AI agent that makes phone calls on your behalf, guided by your
//! values and principles.

mod cli;
mod config;
mod conversation;
mod error;
mod task;
mod telephony;
mod ui;
mod voice;

use cli::{Cli, Commands, ConfigCommands, DemoScenario, TaskCommands};
use config::KartaConfig;
use conversation::ConversationSession;
use error::{KartaError, Result};
use task::{Task, TaskBuilder, TaskType};
use ui::display;

use std::path::PathBuf;

#[tokio::main]
async fn main() {
    // Load environment variables from .env file
    if let Err(e) = dotenvy::dotenv() {
        // It's okay if .env doesn't exist, we'll use config file or defaults
        tracing::debug!("No .env file loaded: {}", e);
    }

    // Initialize logging
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::from_default_env()
                .add_directive(tracing::Level::INFO.into()),
        )
        .init();

    // Parse CLI arguments
    let cli = Cli::parse_args();

    // Run the appropriate command
    if let Err(e) = run(cli).await {
        display::print_error(&format!("{}", e));
        std::process::exit(1);
    }
}

async fn run(cli: Cli) -> Result<()> {
    match cli.command {
        Commands::Call(args) => run_call(cli.config, cli.mock, args).await,
        Commands::Test(args) => run_test_call(args).await,
        Commands::Task(args) => run_task(args).await,
        Commands::Config(args) => run_config(args).await,
        Commands::Demo(args) => run_demo(args).await,
    }
}

/// Run a call
async fn run_call(
    config_path: Option<PathBuf>,
    mock_mode: bool,
    args: cli::CallArgs,
) -> Result<()> {
    display::print_banner();

    // Load configuration
    let mut config = load_config(config_path)?;

    // Override to mock mode if requested
    if mock_mode {
        config.telephony.provider = config::TelephonyProvider::Mock;
        config.voice.provider = config::VoiceProvider::Mock;
    }

    // Build the task
    let task = TaskBuilder::new()
        .task_type(args.task.into())
        .description(args.goal.clone().unwrap_or_else(|| {
            format!("Contact {}", args.target)
        }))
        .target_with_context(
            &args.target,
            args.phone,
            args.context.unwrap_or_default(),
        )
        .goal(args.goal.unwrap_or_else(|| "Complete the task".into()))
        .flexible(args.flexible)
        .firm(args.firm)
        .budget_ceiling(args.budget_ceiling.unwrap_or(0.0))
        .build()
        .map_err(|e| KartaError::Task(e))?;

    display::print_task_summary(&task);
    display::print_separator();

    // Create providers
    let telephony = telephony::create_provider(&config.telephony)?;
    let voice_engine = voice::create_engine(&config.voice)?;

    // Create and run the session
    let mut session = ConversationSession::new(task, config);

    // Get channels for the UI
    let ui_rx = session.ui_events();
    let input_tx = session.input_sender();

    // Spawn the conversation in a separate task
    let session_handle = tokio::spawn(async move {
        session.run(telephony, voice_engine).await
    });

    // Run the backchannel UI
    let mut backchannel = ui::Backchannel::new(ui_rx, input_tx);
    backchannel.run().await?;

    // Wait for session to complete
    session_handle.await.map_err(|e| KartaError::Task(e.to_string()))??;

    display::print_separator();
    display::print_status("Session complete");

    Ok(())
}

/// Run a quick test call
async fn run_test_call(args: cli::TestCallArgs) -> Result<()> {
    display::print_banner();

    println!("{}📞 Making test call to: {}{}\n",
        display::colors::CYAN,
        args.phone,
        display::colors::RESET
    );

    // Load config to get Twilio credentials
    let config = load_config(None)?;

    // Check if Twilio is configured
    if config.telephony.provider != config::TelephonyProvider::Twilio {
        return Err(KartaError::Config(
            "Twilio not configured. Set TWILIO_ACCOUNT_SID, TWILIO_AUTH_TOKEN, and TWILIO_PHONE_NUMBER in .env".into()
        ));
    }

    let account_sid = config.telephony.twilio_account_sid.clone()
        .ok_or_else(|| KartaError::Config("TWILIO_ACCOUNT_SID not set".into()))?;
    let auth_token = config.telephony.twilio_auth_token.clone()
        .ok_or_else(|| KartaError::Config("TWILIO_AUTH_TOKEN not set".into()))?;
    let from_number = config.telephony.twilio_phone_number.clone()
        .ok_or_else(|| KartaError::Config("TWILIO_PHONE_NUMBER not set".into()))?;

    println!("{}From: {}{}", display::colors::DIM, from_number, display::colors::RESET);
    println!("{}Message: {}{}\n", display::colors::DIM, args.message, display::colors::RESET);

    // Create Twilio provider and make the call
    let provider = telephony::TwilioProvider::new(account_sid, auth_token, from_number);

    display::print_status("Initiating call...");

    match provider.create_twilio_call_with_message(&args.phone, &args.message).await {
        Ok(call_sid) => {
            display::print_success(&format!("Call initiated! SID: {}", call_sid));
            println!("\n{}The phone should ring momentarily...{}",
                display::colors::GREEN, display::colors::RESET);
        }
        Err(e) => {
            display::print_error(&format!("Call failed: {}", e));
            return Err(e);
        }
    }

    Ok(())
}

/// Run task management commands
async fn run_task(args: cli::TaskArgs) -> Result<()> {
    let memory = task::TaskMemory::default_location()?;

    match args.command {
        TaskCommands::List { all, limit } => {
            let tasks = if all {
                memory.recent(limit)
            } else {
                memory.list_active()
            };

            if tasks.is_empty() {
                println!("No tasks found.");
            } else {
                println!("Tasks:");
                for task in tasks {
                    println!("  {} - {}", &task.id.to_string()[..8], task.summary());
                }
            }
        }

        TaskCommands::Show { id } => {
            // Find task by partial ID match
            let tasks: Vec<_> = memory.list().into_iter()
                .filter(|t| t.id.to_string().starts_with(&id))
                .collect();

            match tasks.len() {
                0 => println!("No task found matching '{}'", id),
                1 => {
                    let task = tasks[0];
                    println!("{}", serde_json::to_string_pretty(task)?);
                }
                _ => {
                    println!("Multiple tasks match '{}':", id);
                    for task in tasks {
                        println!("  {}", task.id);
                    }
                }
            }
        }

        TaskCommands::Search { query } => {
            let tasks = memory.search_by_target(&query);

            if tasks.is_empty() {
                println!("No tasks found matching '{}'", query);
            } else {
                println!("Tasks matching '{}':", query);
                for task in tasks {
                    println!("  {} - {}", &task.id.to_string()[..8], task.summary());
                }
            }
        }
    }

    Ok(())
}

/// Run configuration commands
async fn run_config(args: cli::ConfigArgs) -> Result<()> {
    match args.command {
        ConfigCommands::Init { force } => {
            let config_path = KartaConfig::default_config_path();

            if config_path.exists() && !force {
                return Err(KartaError::Config(format!(
                    "Config already exists at {}. Use --force to overwrite.",
                    config_path.display()
                )));
            }

            // Create default config
            let config = KartaConfig::default_config();
            config.save(&config_path)?;

            println!("Configuration created at: {}", config_path.display());
            println!();
            println!("Next steps:");
            println!("1. Edit the config to add your information");
            println!("2. Add API keys for Twilio and Gemini (or use --mock mode)");
            println!("3. Run 'karta demo' to test");
        }

        ConfigCommands::Show => {
            let config = KartaConfig::load_default()?;
            let toml = toml::to_string_pretty(&config)
                .map_err(|e| KartaError::Config(e.to_string()))?;
            println!("{}", toml);
        }

        ConfigCommands::Edit => {
            let config_path = KartaConfig::default_config_path();
            let editor = std::env::var("EDITOR").unwrap_or_else(|_| "vim".into());

            std::process::Command::new(&editor)
                .arg(&config_path)
                .status()
                .map_err(|e| KartaError::Config(format!("Failed to open editor: {}", e)))?;
        }

        ConfigCommands::Validate => {
            match KartaConfig::load_default() {
                Ok(config) => {
                    println!("✓ Configuration is valid");
                    println!("  Principal: {}", config.principal.name);
                    println!("  Agent: {}", config.agent.name);
                    println!("  Telephony: {:?}", config.telephony.provider);
                    println!("  Voice: {:?}", config.voice.provider);
                }
                Err(e) => {
                    println!("✗ Configuration error: {}", e);
                }
            }
        }

        ConfigCommands::Path => {
            println!("{}", KartaConfig::default_config_path().display());
        }
    }

    Ok(())
}

/// Run demo mode
async fn run_demo(args: cli::DemoArgs) -> Result<()> {
    display::print_banner();

    println!("{}Running demo: {:?}{}\n",
        display::colors::CYAN,
        args.scenario,
        display::colors::RESET
    );

    // Create a demo config
    let mut config = KartaConfig::default_config();
    config.principal.name = "Demo User".into();
    config.telephony.provider = config::TelephonyProvider::Mock;
    config.voice.provider = config::VoiceProvider::Mock;

    // Create a demo task based on scenario
    let task = match args.scenario {
        DemoScenario::Appointment => {
            TaskBuilder::new()
                .task_type(TaskType::BookAppointment)
                .description("Book a doctor's appointment")
                .target("Acme Medical Center", Some("+1-555-0123".into()))
                .goal("Schedule a checkup appointment for next week")
                .flexible(vec!["specific day".into(), "morning vs afternoon".into()])
                .firm(vec!["must be next week".into()])
                .build()
                .unwrap()
        }
        DemoScenario::Rental => {
            TaskBuilder::new()
                .task_type(TaskType::RentalAgreement)
                .description("Negotiate rental agreement")
                .target_with_context(
                    "Apex Properties",
                    Some("+1-555-0456".into()),
                    "123 Oak Street, 1BR apartment"
                )
                .goal("Negotiate rental price and complete agreement")
                .budget_ceiling(2650.0)
                .flexible(vec!["lease length".into(), "move-in date".into()])
                .firm(vec!["no parking fee".into(), "price under $2,650".into()])
                .build()
                .unwrap()
        }
    };

    display::print_task_summary(&task);
    display::print_separator();

    // Set up the mock voice engine with the appropriate flow
    let mut mock_voice: voice::MockVoiceEngine = voice::MockVoiceEngine::new();
    match args.scenario {
        DemoScenario::Appointment => mock_voice.setup_appointment_flow(),
        DemoScenario::Rental => mock_voice.setup_rental_flow(),
    }

    let telephony = telephony::create_provider(&config.telephony)?;

    // Create and run the session
    let mut session = ConversationSession::new(task, config);

    let ui_rx = session.ui_events();
    let input_tx = session.input_sender();

    // Spawn the conversation
    let session_handle = tokio::spawn(async move {
        session.run(telephony, Box::new(mock_voice)).await
    });

    // Run simple display for demo
    let mut display = ui::SimpleDisplay::new(ui_rx);
    display.run().await?;

    session_handle.await.map_err(|e| KartaError::Task(e.to_string()))??;

    display::print_separator();
    display::print_success("Demo complete!");

    Ok(())
}

/// Load configuration from file or default location
fn load_config(path: Option<PathBuf>) -> Result<KartaConfig> {
    match path {
        Some(p) => KartaConfig::load(&p),
        None => {
            // Try default location, fall back to default config
            match KartaConfig::load_default() {
                Ok(config) => Ok(config),
                Err(_) => {
                    tracing::info!("No config found, using defaults");
                    Ok(KartaConfig::default_config())
                }
            }
        }
    }
}
