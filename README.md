# Rust Discord Bot with Shuttle

This repository is a Discord bot built using Rust's Serenity. Hosting is provided by Shuttle.

Since it is still under development, it has quite a few features, but the following functions are available:
- When you make a mentions to the Bot, it gets the 5 latest messages and Claude 3.5 Sonnet will answer them.
- Messages longer than 2000 characters are automatically split into multiple messages to comply with Discord's message length limit.
- When a URL of x.com or twitter.com is pasted, it is converted to vxtwitter.com and posted

## Requirements

- Rust (stable channel)
- Cargo
- cargo-shuttle

> [!IMPORTANT]
> This project uses Rust edition 2021. The Cargo.toml file originally specified edition 2024, but this has been changed to 2021 for compatibility with the current stable version of Cargo.

## Install & Build

Install various required packages using Cargo. Build is also performed at the same time.

```bash
cargo build
```

## Deploy

### Get a Discord token

To run this bot, you need a valid Discord token; login to the [Discord Developer Portal](https://discord.com/developers/applications).

The required authority is **"MESSAGE CONTENT INTENT"**

> [!NOTE]
> The official Shuttle website describes [how to install the Hello world bot](https://docs.shuttle.rs/examples/serenity), but it also describes the operation of the Discord Developers Portal, so please refer to that if you are not sure.


### Get an Anthropic Claude token

To run this bot, you need a valid Anthropic Claude API token; login to the [Anthropic Console](https://console.anthropic.com/) and create an API key.

> [!WARNING]
> You must have a valid Anthropic account to use Claude API.

### Install Shuttle CLI

Please install it beforehand, as it will be installed using cargo-binstall.  
Install the cargo-shuttle binary.

```bash
cargo install cargo-shuttle
```

Log in to Shuttle.  
You will be asked to enter your API key, which can be obtained from the following location after logging into your browser dashboard: https://console.shuttle.rs/account/overview

```bash
cargo shuttle login
```

### Write various tokens in Secrets.toml

```toml
DISCORD_TOKEN="{{ token }}"
CLAUDE_TOKEN="{{ token }}"
```

> [!TIP]
> Shuttle refers to the value in Secrets.toml, not the env file.

### Deploy to Shuttle

#### Init

Initial project setup for Shuttle. Here you can specify the project name, etc.

```bash
cargo shuttle init
```

#### Project Start

```bash
shuttle project new --idle-minutes 0
```

> [!TIP]
> Since the application sleeps after 30 minutes of inactivity, the `--idle-minutes` option can be used to set the application not to sleep when the project is created.

#### Deploy

Use the `--allow-dirty` option to upload Secrets.toml to Shuttle, which will not be pushed to Git the first time.
https://docs.shuttle.rs/getting-started/shuttle-commands

```bash
cargo shuttle deploy --allow-dirty
```

> [!NOTE]
> If you encounter an error related to Rust edition 2024, you may need to modify the `Cargo.toml` file to use edition 2021 instead:
> ```toml
> [package]
> name = "ideal-x"
> version = "1.0.0"
> edition = "2021"  # Changed from 2024
> ```

### 🎉 Completed

Check the online status of the bot you let join the Discord, and try mentions to see how it works!

The bot should be accessible at: `https://[project-name].shuttle.app`