# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Project Overview

IdealX is a Discord bot written in Rust that integrates with Anthropic's Claude AI. The bot provides intelligent responses in Discord servers and is deployed on the Shuttle hosting platform.

## Development Commands

### Build and Test
```bash
# Build the project
cargo build

# Run locally (requires valid tokens in Secrets.toml)
cargo run

# Format code
cargo fmt

# Lint code
cargo clippy
```

### Deployment
```bash
# Deploy to Shuttle (first time or when secrets change)
cargo shuttle deploy --allow-dirty

# Regular deployment
cargo shuttle deploy

# Initialize new Shuttle project
cargo shuttle init
```

## Architecture

### Core Framework Stack
- **Discord Integration**: Serenity framework with Poise command system
- **AI Integration**: Anthropic Claude API (claude-sonnet-4-20250514)
- **Async Runtime**: Tokio
- **Hosting**: Shuttle platform

### Key Modules
- `src/main.rs`: Main bot logic, event handlers, and Shuttle runtime
- `src/claude.rs`: Claude API client with message splitting utilities
- `src/commands/`: Bot slash commands (example: age calculation)

### Configuration
- `Secrets.toml`: Contains Discord and Claude API tokens, plus target server/channel IDs for auto-response feature
- Auto-response is enabled for specific forum channels defined in `TARGET_FORUM_CHANNEL_IDS`

## Bot Features

### Message Handling
- Responds to mentions using Claude AI with conversation context (last 5 messages)
- Auto-responds in configured forum channels without requiring mentions
- Automatically splits responses >2000 characters to comply with Discord limits
- Converts Twitter/X URLs to vxtwitter.com format

### Technical Implementation
- Event-driven architecture using Serenity's event handlers
- Message context gathering via `get_channel_messages()`
- Intelligent message splitting in `split_message()` function
- Server/channel targeting via configuration-driven approach

## Important Notes

### Secrets Management
- Never commit `Secrets.toml` - it contains sensitive API tokens
- Shuttle deployment requires `--allow-dirty` flag when secrets are present
- Bot requires "MESSAGE CONTENT INTENT" permission in Discord Developer Portal

### Dependencies
- Uses Rust edition 2021 (not 2024) for compatibility
- Key dependencies: shuttle-serenity, serenity, poise, reqwest, tokio
- Built for Shuttle serverless platform with automatic scaling