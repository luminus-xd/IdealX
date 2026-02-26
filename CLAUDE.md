# CLAUDE.md

Answer in Japanese.

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Project Overview

IdealX is a Discord bot built with Vercel Chat SDK and AI SDK that integrates with Anthropic's Claude AI. The bot provides intelligent responses in Discord servers and is deployed on the Railway hosting platform.

## Development Commands

### Build and Run
```bash
# Install dependencies
npm install

# Run in development mode (with hot reload)
npm run dev

# Build for production
npm run build

# Run production build
npm start

# Register Discord slash commands (first time only)
npm run register
```

### Deployment
```bash
# Deploy to Railway
# 1. Connect GitHub repository to Railway
# 2. Set environment variables in Railway dashboard
# 3. Deploy automatically on git push

# Local testing with environment variables
cp .env.example .env
# Edit .env with your tokens
npm run dev
```

## Architecture

### Core Framework Stack
- **Bot Framework**: Vercel Chat SDK (`chat` + `@chat-adapter/discord`)
- **AI Integration**: AI SDK (`ai` + `@ai-sdk/anthropic`) with Claude claude-sonnet-4-6
- **HTTP Server**: Hono with @hono/node-server
- **Hosting**: Railway platform

### Key Modules
- `src/index.ts`: Hono server, webhook endpoint, gateway listener startup
- `src/lib/bot.ts`: Chat SDK bot instance, all event handlers (mention, forum, reaction, commands)
- `src/lib/ai.ts`: AI SDK integration (generateText with web search, summarize, translate)
- `src/lib/adapters.ts`: Discord adapter configuration
- `src/register-commands.ts`: Discord slash command registration script

### Configuration
- **Environment Variables**:
  - `DISCORD_BOT_TOKEN`: Discord bot token
  - `DISCORD_PUBLIC_KEY`: Discord Ed25519 public key for webhook verification
  - `DISCORD_APPLICATION_ID`: Discord application ID
  - `ANTHROPIC_API_KEY`: Anthropic Claude API key
  - `TARGET_SERVER_IDS`: Comma-separated list of Discord server IDs for auto-response
  - `TARGET_FORUM_CHANNEL_IDS`: Comma-separated list of forum channel IDs for auto-response
  - `PORT`: Server port (default: 3000)

## Bot Features

### Event Handlers
- `onNewMention`: Responds to mentions with Claude AI (last 5 messages context)
- `onSubscribedMessage`: Handles follow-up messages in subscribed threads/forums
- `onNewMessage(/.*/s)`: Auto-responds in configured forum channels
- `onNewMessage(/ぬるぽ/)`: Easter egg
- `onReaction(['📝'])`: Summarizes reacted message and URL contents
- `onSlashCommand`: help, age, summarize, translate, clear

### Technical Implementation
- Webhook-based architecture for Discord interactions
- Gateway listener for real-time message/reaction events
- AI SDK generateText with Anthropic web search tool (maxSteps: 6)
- Intelligent message splitting for Discord's 2000-char limit

## Important Notes

### Environment Variables Management
- Never commit API tokens to version control
- Use `.env` file for local development
- Set environment variables in Railway dashboard for production

### Discord Setup
- Bot requires "MESSAGE CONTENT INTENT" in Discord Developer Portal
- Set Interactions Endpoint URL to `https://<domain>/api/webhooks/discord`
- Run `npm run register` to register slash commands

### Railway Deployment
- Dockerfile included for containerized deployment
- Multi-stage Node.js build for optimized container size
- Automatic deployment on git push
