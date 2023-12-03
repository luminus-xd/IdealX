use anyhow::anyhow;
use serenity::async_trait;
use serenity::model::channel::Message;
use serenity::model::gateway::Ready;
use serenity::prelude::*;
use shuttle_secrets::SecretStore;
use tracing::{error, info};
use regex::Regex;

struct Bot;

#[async_trait]
impl EventHandler for Bot {
    async fn message(&self, ctx: Context, msg: Message) {
        if msg.content == "ぬるぽ" {
            if let Err(e) = msg.channel_id.say(&ctx.http, "ガッ").await {
                error!("Error sending message: {:?}", e);
            }
        }

        if msg.content == "!hello" {
            if let Err(e) = msg.channel_id.say(&ctx.http, "world!").await {
                error!("Error sending message: {:?}", e);
            }
        }

        if msg.content.contains("https://x.com") || msg.content.contains("https://twitter.com") {
            let re = match Regex::new(r"(https://(x\.com|twitter\.com)[^\s]+)") {
                Ok(re) => re,
                Err(err) => {
                    error!("Error creating regex: {:?}", err);
                    return;
                }
            };
            let mut replaced_urls = Vec::new();
            for cap in re.captures_iter(&msg.content) {
                replaced_urls.push(
                    cap[0]
                        .replace("twitter.com", "vxtwitter.com")
                        .replace("x.com", "vxtwitter.com"),
                );
            }
            let urls_string = replaced_urls.join("\n");
            if let Err(e) = msg.channel_id.say(&ctx.http, urls_string).await {
                error!("Error sending message: {:?}", e);
            }
        }
    }

    async fn ready(&self, _: Context, ready: Ready) {
        info!("{} is connected!", ready.user.name);
    }
}

#[shuttle_runtime::main]
async fn serenity(
    #[shuttle_secrets::Secrets] secret_store: SecretStore,
) -> shuttle_serenity::ShuttleSerenity {
    // Get the discord token set in `Secrets.toml`
    let token = if let Some(token) = secret_store.get("DISCORD_TOKEN") {
        token
    } else {
        return Err(anyhow!("'DISCORD_TOKEN' was not found").into());
    };

    // Set gateway intents, which decides what events the bot will be notified about
    let intents = GatewayIntents::GUILD_MESSAGES | GatewayIntents::MESSAGE_CONTENT;

    let client = Client::builder(&token, intents)
        .event_handler(Bot)
        .await
        .expect("Err creating client");

    Ok(client.into())
}
