# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Project Overview

Karta is a voice-native AI executive assistant CLI written in Rust. It makes phone calls on behalf of a user (the "principal"), guided by their values and preferences. The user supervises calls via a real-time text backchannel in the terminal.

## Build Commands

```bash
# Build
cargo build

# Build release
cargo build --release

# Run in development
cargo run -- <args>

# Run tests
cargo test

# Check formatting
cargo fmt --check

# Run lints
cargo clippy

# Run a specific test
cargo test <test_name>
```

## Running

```bash
# Initialize config
cargo run -- config init

# Run demos (no API keys needed)
cargo run -- demo appointment
cargo run -- demo rental

# Make a call in mock mode
cargo run -- --mock call "Target" --phone "+1-555-0123" --task inquiry
```

## Architecture

The codebase follows a modular architecture with clear separation of concerns:

### Core Modules

- **`cli.rs`** - Clap-based CLI argument parsing
- **`config/`** - Configuration system
  - `principal.rs` - User's values, boundaries, and preferences (the "soul" of the system)
  - `agent.rs` - Agent personality and behavior configuration
- **`task/`** - Task management
  - `task.rs` - Task struct, builder, and state machine
  - `memory.rs` - Task persistence to JSON files
- **`telephony/`** - Phone call abstraction
  - `provider.rs` - Trait defining telephony providers
  - `mock.rs` - Mock provider for development
  - `twilio.rs` - Twilio integration (stub ready for implementation)
- **`voice/`** - Voice/AI conversation engine
  - `engine.rs` - Trait defining voice engines
  - `mock.rs` - Mock engine with scripted conversations
  - `gemini.rs` - Gemini Live WebSocket integration
- **`conversation/`** - Call orchestration
  - `state.rs` - Conversation state machine
  - `session.rs` - Coordinates telephony, voice, and UI
- **`ui/`** - Terminal interface
  - `display.rs` - ANSI color output helpers
  - `backchannel.rs` - Real-time terminal UI with input

### Key Design Patterns

1. **Trait-based providers** - Telephony and voice engines use traits (`TelephonyProvider`, `VoiceEngine`) allowing mock implementations for testing and real implementations for production.

2. **Async channels** - Communication between conversation session and UI uses `async_channel` for real-time transcript streaming.

3. **State machine** - `ConversationStateMachine` manages call lifecycle with explicit state transitions.

4. **Builder pattern** - `TaskBuilder` for constructing tasks with optional parameters.

5. **TOML configuration** - Principal profile and agent config stored in `~/.config/karta/config.toml`.

### Data Flow

```
CLI Args → Task → ConversationSession
                         ↓
              ┌──────────┴──────────┐
              ↓                     ↓
    TelephonyProvider         VoiceEngine
    (make/end calls)     (process audio/text)
              ↓                     ↓
              └──────────┬──────────┘
                         ↓
                 UIEvent channel
                         ↓
                   Backchannel
                  (terminal UI)
```

## API Keys

The system supports multiple providers configured in `config.toml`:

- **Telephony**: `mock` (default) or `twilio`
- **Voice**: `mock` (default), `gemini`, or `openai`

Mock mode works without any API keys for development.

## Key Files for Common Tasks

- Adding a new CLI command: `src/cli.rs`
- Modifying principal boundaries: `src/config/principal.rs`
- Adding agent personality options: `src/config/agent.rs`
- New telephony provider: Implement `TelephonyProvider` trait in `src/telephony/`
- New voice engine: Implement `VoiceEngine` trait in `src/voice/`
- Modify conversation flow: `src/conversation/session.rs`
- Change terminal output: `src/ui/display.rs`

## Dependencies

Key crates:
- `tokio` - Async runtime
- `clap` - CLI parsing
- `serde`/`toml`/`serde_json` - Serialization
- `tokio-tungstenite` - WebSocket for Gemini Live
- `crossterm`/`ratatui` - Terminal UI
- `async-channel` - Async communication
- `chrono` - Timestamps
- `uuid` - Task IDs
