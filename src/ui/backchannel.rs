//! Backchannel - the terminal interface for real-time interaction

use async_channel::{Receiver, Sender};
use crossterm::{
    event::{self, Event, KeyCode, KeyEvent, KeyModifiers},
    terminal::{disable_raw_mode, enable_raw_mode},
};
use std::io::{self, Write};
use std::time::Duration;

use crate::conversation::{ConversationState, UIEvent, WaitingContext};
use crate::error::Result;
use crate::ui::display;

/// The backchannel terminal interface
pub struct Backchannel {
    /// Input buffer
    input_buffer: String,

    /// Channel to receive UI events
    ui_rx: Receiver<UIEvent>,

    /// Channel to send principal input
    input_tx: Sender<String>,

    /// Whether we're waiting for input
    waiting_for_input: bool,

    /// Current waiting context
    waiting_context: Option<WaitingContext>,

    /// Transcript history
    transcript: Vec<String>,

    /// Max transcript lines to keep
    max_transcript_lines: usize,

    /// Whether the session is active
    active: bool,
}

impl Backchannel {
    /// Create a new backchannel interface
    pub fn new(ui_rx: Receiver<UIEvent>, input_tx: Sender<String>) -> Self {
        Backchannel {
            input_buffer: String::new(),
            ui_rx,
            input_tx,
            waiting_for_input: false,
            waiting_context: None,
            transcript: Vec::new(),
            max_transcript_lines: 100,
            active: true,
        }
    }

    /// Run the backchannel interface
    pub async fn run(&mut self) -> Result<()> {
        // Set up terminal
        enable_raw_mode()?;
        let mut stdout = io::stdout();

        // Print initial prompt
        self.render_status("Ready. Type a message or wait for Karta to ask for input.")?;

        loop {
            // Check for UI events
            while let Ok(event) = self.ui_rx.try_recv() {
                self.handle_ui_event(event)?;
            }

            // Check for keyboard input
            if event::poll(Duration::from_millis(50))? {
                if let Event::Key(key_event) = event::read()? {
                    if !self.handle_key_event(key_event).await? {
                        break;
                    }
                }
            }

            if !self.active {
                break;
            }
        }

        // Restore terminal
        disable_raw_mode()?;
        println!();

        Ok(())
    }

    /// Handle a UI event
    fn handle_ui_event(&mut self, event: UIEvent) -> Result<()> {
        match event {
            UIEvent::Status(msg) => {
                self.render_line(&format!("\r\n{}[STATUS]{} {}\r\n",
                    display::colors::BLUE,
                    display::colors::RESET,
                    msg
                ))?;
            }

            UIEvent::Transcript(transcript) => {
                let formatted = display::format_transcript(&transcript);
                self.transcript.push(formatted.clone());

                // Trim transcript if too long
                if self.transcript.len() > self.max_transcript_lines {
                    self.transcript.remove(0);
                }

                self.render_line(&format!("\r\n{}\r\n", formatted))?;
            }

            UIEvent::NeedInput(ctx) => {
                self.waiting_for_input = true;
                self.waiting_context = Some(ctx.clone());
                self.render_question(&ctx)?;
            }

            UIEvent::CallStateChanged(state) => {
                let state_str = display::format_state(&state);
                self.render_line(&format!("\r\n{}\r\n", state_str))?;
            }

            UIEvent::TaskCompleted(success, msg) => {
                if success {
                    self.render_line(&format!("\r\n{}[SUCCESS]{} {}\r\n",
                        display::colors::GREEN,
                        display::colors::RESET,
                        msg
                    ))?;
                } else {
                    self.render_line(&format!("\r\n{}[INCOMPLETE]{} {}\r\n",
                        display::colors::YELLOW,
                        display::colors::RESET,
                        msg
                    ))?;
                }
                self.active = false;
            }

            UIEvent::Error(msg) => {
                self.render_line(&format!("\r\n{}[ERROR]{} {}\r\n",
                    display::colors::RED,
                    display::colors::RESET,
                    msg
                ))?;
            }
        }

        // Re-render input line if we have content
        if !self.input_buffer.is_empty() {
            self.render_input_line()?;
        }

        Ok(())
    }

    /// Handle a key event
    async fn handle_key_event(&mut self, key: KeyEvent) -> Result<bool> {
        match key.code {
            // Ctrl+C to quit
            KeyCode::Char('c') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                self.render_line("\r\n\nExiting...\r\n")?;
                return Ok(false);
            }

            // Enter to submit
            KeyCode::Enter => {
                if !self.input_buffer.is_empty() {
                    let input = self.input_buffer.clone();
                    self.input_buffer.clear();

                    // Show what was typed
                    self.render_line(&format!(
                        "\r\n{}[YOU]{} {}\r\n",
                        display::colors::GREEN,
                        display::colors::RESET,
                        input
                    ))?;

                    // Send input
                    self.input_tx.send(input).await.ok();
                    self.waiting_for_input = false;
                    self.waiting_context = None;
                }
            }

            // Backspace
            KeyCode::Backspace => {
                if !self.input_buffer.is_empty() {
                    self.input_buffer.pop();
                    self.render_input_line()?;
                }
            }

            // Regular character
            KeyCode::Char(c) => {
                self.input_buffer.push(c);
                self.render_input_line()?;
            }

            // Escape to clear input
            KeyCode::Esc => {
                self.input_buffer.clear();
                self.render_input_line()?;
            }

            _ => {}
        }

        Ok(true)
    }

    fn render_line(&self, text: &str) -> Result<()> {
        let mut stdout = io::stdout();
        // Clear current line, move to beginning, print text
        write!(stdout, "\r\x1B[K{}", text)?;
        stdout.flush()?;
        Ok(())
    }

    fn render_status(&self, msg: &str) -> Result<()> {
        let mut stdout = io::stdout();
        write!(stdout, "\r\n{}[STATUS]{} {}\r\n",
            display::colors::BLUE,
            display::colors::RESET,
            msg
        )?;
        write!(stdout, "{}>{} ", display::colors::GREEN, display::colors::RESET)?;
        stdout.flush()?;
        Ok(())
    }

    fn render_input_line(&self) -> Result<()> {
        let mut stdout = io::stdout();
        write!(stdout, "\r\x1B[K{}>{} {}",
            display::colors::GREEN,
            display::colors::RESET,
            self.input_buffer
        )?;
        stdout.flush()?;
        Ok(())
    }

    fn render_question(&self, ctx: &WaitingContext) -> Result<()> {
        let mut stdout = io::stdout();

        write!(stdout, "\r\n\n{}💬 KARTA NEEDS INPUT:{}\r\n",
            display::colors::BG_YELLOW,
            display::colors::RESET
        )?;
        write!(stdout, "   {}\r\n", ctx.question)?;

        if !ctx.options.is_empty() {
            write!(stdout, "\r\n   Suggested responses:\r\n")?;
            for (i, option) in ctx.options.iter().enumerate() {
                write!(stdout, "   {}[{}]{} {}\r\n",
                    display::colors::DIM,
                    i + 1,
                    display::colors::RESET,
                    option
                )?;
            }
        }

        write!(stdout, "\r\n{}>{} ", display::colors::GREEN, display::colors::RESET)?;
        stdout.flush()?;

        Ok(())
    }
}

/// Simple non-interactive display (for when we can't use raw mode)
pub struct SimpleDisplay {
    ui_rx: Receiver<UIEvent>,
}

impl SimpleDisplay {
    pub fn new(ui_rx: Receiver<UIEvent>) -> Self {
        SimpleDisplay { ui_rx }
    }

    pub async fn run(&mut self) -> Result<()> {
        while let Ok(event) = self.ui_rx.recv().await {
            match event {
                UIEvent::Status(msg) => {
                    display::print_status(&msg);
                }
                UIEvent::Transcript(transcript) => {
                    display::print_transcript(&transcript);
                }
                UIEvent::NeedInput(ctx) => {
                    display::print_question(&ctx.question, &ctx.options);
                }
                UIEvent::CallStateChanged(state) => {
                    println!("{}", display::format_state(&state));
                }
                UIEvent::TaskCompleted(success, msg) => {
                    if success {
                        display::print_success(&msg);
                    } else {
                        display::print_warning(&msg);
                    }
                    break;
                }
                UIEvent::Error(msg) => {
                    display::print_error(&msg);
                }
            }
        }

        Ok(())
    }
}
