# CLAUDE.md

Answer in Japanese.

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Project Overview

IdealX is a Discord bot written in Rust that integrates with Anthropic's Claude AI. The bot provides intelligent responses in Discord servers and is deployed on the Railway hosting platform.

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
# Deploy to Railway
# 1. Connect GitHub repository to Railway
# 2. Set environment variables in Railway dashboard
# 3. Deploy automatically on git push

# Local testing with environment variables
export DISCORD_TOKEN="your_discord_token"
export CLAUDE_TOKEN="your_claude_token"
export TARGET_SERVER_IDS="server_id1,server_id2"
export TARGET_FORUM_CHANNEL_IDS="channel_id1,channel_id2"
cargo run
```

## Architecture

### Core Framework Stack
- **Discord Integration**: Serenity framework with Poise command system
- **AI Integration**: Anthropic Claude API (claude-sonnet-4-20250514)
- **Async Runtime**: Tokio
- **Hosting**: Railway platform

### Key Modules
- `src/main.rs`: Main bot logic, event handlers, and main runtime
- `src/claude.rs`: Claude API client with message splitting utilities
- `src/commands/`: Bot slash commands (example: age calculation)

### Configuration
- **Environment Variables**: Contains Discord and Claude API tokens, plus target server/channel IDs for auto-response feature
  - `DISCORD_TOKEN`: Discord bot token
  - `CLAUDE_TOKEN`: Anthropic Claude API token
  - `TARGET_SERVER_IDS`: Comma-separated list of Discord server IDs for auto-response
  - `TARGET_FORUM_CHANNEL_IDS`: Comma-separated list of forum channel IDs for auto-response
- Auto-response is enabled for specific forum channels defined in `TARGET_FORUM_CHANNEL_IDS`

## Bot Features

### Message Handling
- Responds to mentions using Claude AI with conversation context (last 5 messages)
- Auto-responds in configured forum channels without requiring mentions
- Automatically splits responses >2000 characters to comply with Discord limits

### Technical Implementation
- Event-driven architecture using Serenity's event handlers
- Message context gathering via `get_channel_messages()`
- Intelligent message splitting in `split_message()` function
- Server/channel targeting via configuration-driven approach

## Important Notes

### Environment Variables Management
- Never commit environment variables containing sensitive API tokens to version control
- Set environment variables in Railway dashboard for production deployment
- Bot requires "MESSAGE CONTENT INTENT" permission in Discord Developer Portal

### Dependencies
- Uses Rust edition 2021 (not 2024) for compatibility
- Key dependencies: serenity, poise, reqwest, tokio, anyhow
- Built for Railway platform with automatic scaling and container deployment

### Railway Deployment
- Dockerfile included for containerized deployment
- Automatic deployment on git push to connected repository
- Environment variables configured through Railway dashboard
- Multi-stage build for optimized container size