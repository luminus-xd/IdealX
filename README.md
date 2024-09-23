# Rust Discord Bot with Shuttle

This repository is a Discord bot built using Rust's Serenity. Hosting is provided by Shuttle. 

Since it is still under development, it has quite a few features, but the following functions are available
- When you make a mentions to the Bot, it gets the 5 latest messages and GPT will answer them.
- When a URL of x.com or twitter.com is pasted, it is converted to vxtwitter.com and posted

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


### Get a OpenAI token

To run this bot, you need a valid OpenAI token; login to the [OpenAI Profile | User APPI keys](https://platform.openai.com/settings/profile?tab=api-keys).

> [!WARNING]
> You must have charged credits to use OpenAI's ChatGPT API.

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
CHATGPT_TOKEN="{{ token }}"
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

### 🎉 Completed

Check the online status of the bot you let join the Discord, and try mentions to see how it works!