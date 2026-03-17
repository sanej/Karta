//! Display utilities for terminal output

use crate::conversation::ConversationState;
use crate::task::Task;
use crate::voice::{TranscriptEvent, Speaker};

/// ANSI color codes
pub mod colors {
    pub const RESET: &str = "\x1b[0m";
    pub const BOLD: &str = "\x1b[1m";
    pub const DIM: &str = "\x1b[2m";

    pub const RED: &str = "\x1b[31m";
    pub const GREEN: &str = "\x1b[32m";
    pub const YELLOW: &str = "\x1b[33m";
    pub const BLUE: &str = "\x1b[34m";
    pub const MAGENTA: &str = "\x1b[35m";
    pub const CYAN: &str = "\x1b[36m";
    pub const WHITE: &str = "\x1b[37m";

    pub const BG_RED: &str = "\x1b[41m";
    pub const BG_GREEN: &str = "\x1b[42m";
    pub const BG_YELLOW: &str = "\x1b[43m";
    pub const BG_BLUE: &str = "\x1b[44m";
}

/// Print the Karta banner
pub fn print_banner() {
    println!(r#"
{}{}  _  __          _
 | |/ /__ _ _ __| |_ __ _
 | ' // _` | '__| __/ _` |
 | . \ (_| | |  | || (_| |
 |_|\_\__,_|_|   \__\__,_|
{}
 Voice-Native AI Executive Assistant
 {}v0.1.0{}
"#,
        colors::BOLD,
        colors::CYAN,
        colors::RESET,
        colors::DIM,
        colors::RESET
    );
}

/// Print a status message
pub fn print_status(message: &str) {
    println!("{}[STATUS]{} {}", colors::BLUE, colors::RESET, message);
}

/// Print an error message
pub fn print_error(message: &str) {
    eprintln!("{}[ERROR]{} {}", colors::RED, colors::RESET, message);
}

/// Print a success message
pub fn print_success(message: &str) {
    println!("{}[SUCCESS]{} {}", colors::GREEN, colors::RESET, message);
}

/// Print a warning message
pub fn print_warning(message: &str) {
    println!("{}[WARNING]{} {}", colors::YELLOW, colors::RESET, message);
}

/// Format a transcript event for display
pub fn format_transcript(event: &TranscriptEvent) -> String {
    let (prefix, color) = match event.speaker {
        Speaker::Agent => ("KARTA", colors::CYAN),
        Speaker::Remote => ("THEM ", colors::WHITE),
        Speaker::Principal => ("YOU  ", colors::GREEN),
        Speaker::System => ("SYS  ", colors::DIM),
    };

    let marker = if event.is_final { " " } else { "…" };

    format!(
        "{}[{}]{}{} {}",
        color,
        prefix,
        colors::RESET,
        marker,
        event.text
    )
}

/// Print a transcript event
pub fn print_transcript(event: &TranscriptEvent) {
    println!("{}", format_transcript(event));
}

/// Format task summary for display
pub fn format_task_summary(task: &Task) -> String {
    let mut lines = Vec::new();

    lines.push(format!(
        "{}📋 Task:{} {}",
        colors::BOLD,
        colors::RESET,
        task.description
    ));

    lines.push(format!(
        "{}📍 Target:{} {}",
        colors::BOLD,
        colors::RESET,
        task.target.name
    ));

    if let Some(ref phone) = task.target.phone {
        lines.push(format!(
            "{}📞 Phone:{} {}",
            colors::BOLD,
            colors::RESET,
            phone
        ));
    }

    if let Some(ceiling) = task.boundaries.budget_ceiling {
        lines.push(format!(
            "{}💰 Budget ceiling:{} ${}",
            colors::BOLD,
            colors::RESET,
            ceiling
        ));
    }

    if !task.context.flexible.is_empty() {
        lines.push(format!(
            "{}✓ Flexible:{} {}",
            colors::GREEN,
            colors::RESET,
            task.context.flexible.join(", ")
        ));
    }

    if !task.context.firm.is_empty() {
        lines.push(format!(
            "{}✗ Firm:{} {}",
            colors::RED,
            colors::RESET,
            task.context.firm.join(", ")
        ));
    }

    lines.join("\n")
}

/// Print task summary
pub fn print_task_summary(task: &Task) {
    println!("\n{}", format_task_summary(task));
    println!();
}

/// Format conversation state
pub fn format_state(state: &ConversationState) -> String {
    match state {
        ConversationState::NotStarted => format!("{}⚪ Not started{}", colors::DIM, colors::RESET),
        ConversationState::Initiating => format!("{}🔄 Initiating...{}", colors::YELLOW, colors::RESET),
        ConversationState::Connecting => format!("{}📞 Connecting...{}", colors::YELLOW, colors::RESET),
        ConversationState::Active => format!("{}🟢 Active{}", colors::GREEN, colors::RESET),
        ConversationState::WaitingForPrincipal(_) => format!("{}💬 Waiting for your input{}", colors::MAGENTA, colors::RESET),
        ConversationState::OnHold => format!("{}⏸ On hold{}", colors::YELLOW, colors::RESET),
        ConversationState::Ending => format!("{}🔚 Ending...{}", colors::YELLOW, colors::RESET),
        ConversationState::Ended(reason) => {
            let reason_str = match reason {
                crate::conversation::EndReason::Completed => "completed",
                crate::conversation::EndReason::Failed(msg) => msg,
                crate::conversation::EndReason::PrincipalEnded => "you ended",
                crate::conversation::EndReason::RemoteEnded => "they hung up",
                crate::conversation::EndReason::Error(msg) => msg,
                crate::conversation::EndReason::Timeout => "timeout",
            };
            format!("{}⚫ Ended: {}{}", colors::DIM, reason_str, colors::RESET)
        }
    }
}

/// Print a separator line
pub fn print_separator() {
    println!("{}─────────────────────────────────────────{}", colors::DIM, colors::RESET);
}

/// Print the input prompt
pub fn print_input_prompt() {
    print!("{}>{} ", colors::GREEN, colors::RESET);
    use std::io::Write;
    std::io::stdout().flush().ok();
}

/// Print a question requiring input
pub fn print_question(question: &str, options: &[String]) {
    println!();
    println!(
        "{}💬 KARTA NEEDS INPUT:{}",
        colors::BG_YELLOW,
        colors::RESET
    );
    println!("   {}", question);

    if !options.is_empty() {
        println!();
        println!("   Suggested responses:");
        for (i, option) in options.iter().enumerate() {
            println!("   {}[{}]{} {}", colors::DIM, i + 1, colors::RESET, option);
        }
    }

    println!();
    print_input_prompt();
}

/// Clear the screen
pub fn clear_screen() {
    print!("\x1B[2J\x1B[1;1H");
}
