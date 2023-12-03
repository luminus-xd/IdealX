mod chatgpt;

use chatgpt::get_gpt_response;
use chatgpt::RequestMessage;
use regex::Regex;

use anyhow::Context as _;
use serenity::async_trait;
use serenity::model::channel::Message;
use serenity::model::gateway::Ready;
use serenity::model::user::User;
use serenity::prelude::*;
use serenity::utils::MessageBuilder;
use shuttle_secrets::SecretStore;
use tracing::{error, info};

struct Bot {
    gpt_token: String,
    client: reqwest::Client,
}

fn is_user(author: &User) -> bool {
    return !author.bot;
}

fn is_inclued_bot_mention(ctx: &Context, message: &Message) -> bool {
    return message
        .mentions
        .iter()
        .any(|user| user.id == ctx.cache.current_user_id());
}

fn build_json(messages: Vec<Message>) -> Vec<RequestMessage<'static>> {
    let mention_regexp = Regex::new(r"<@(\d+)>").unwrap();
    return messages
        .iter()
        .rev()
        .map(|message| {
            let content = mention_regexp.replace_all(&message.content, "").to_string();

            let role = match is_user(&message.author) {
                true => "user",
                _ => "assistant",
            };
            RequestMessage { role, content }
        })
        .collect();
}

#[async_trait]
impl EventHandler for Bot {
    async fn message(&self, ctx: Context, msg: Message) {
        if is_inclued_bot_mention(&ctx, &msg) && is_user(&msg.author) {
            let channel_id = msg.channel_id;
            let messages = match channel_id.messages(&ctx.http, |m| m.limit(100)).await {
                Ok(messages) => messages,
                Err(e) => {
                    println!("Error fetching messages: {}", e);
                    return;
                }
            };

            let requset_body: Vec<RequestMessage> = build_json(messages);
            let _typing = match msg.channel_id.start_typing(&ctx.http) {
                Ok(typing) => typing,
                Err(e) => {
                    println!("Error: {}", e);
                    return;
                }
            };
            let gpt_message =
                match get_gpt_response(requset_body, &self.gpt_token, &self.client).await {
                    Ok(text) => text,
                    Err(e) => {
                        println!("Error GPT response: {}", e);
                        return;
                    }
                };
            let response = MessageBuilder::new()
                .mention(&msg.author)
                .push("\n")
                .push(&gpt_message)
                .build();
            if let Err(why) = msg.channel_id.say(&ctx.http, &response).await {
                println!("Error sending message: {:?}", why);
            }
        }

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

    #[cfg(feature = "cache")]
    async fn cache_ready(&self, _ctx: Context, _guilds: Vec<GuildId>) {
        println!("cache ready");
    }
}

#[shuttle_runtime::main]
async fn serenity(
    #[shuttle_secrets::Secrets] secret_store: SecretStore,
) -> shuttle_serenity::ShuttleSerenity {
    let discord_token = secret_store
        .get("DISCORD_TOKEN")
        .context("'DISCORD_TOKEN' was not found")?;
    let gpt_token = secret_store
        .get("CHATGPT_TOKEN")
        .context("'CHATGPT_TOKEN' was not found")?;

    let client = get_client(&discord_token, &gpt_token).await;

    Ok(client.into())
}

pub async fn get_client(discord_token: &str, gpt_token: &str) -> serenity::Client {
    let intents = GatewayIntents::GUILD_MESSAGES
        | GatewayIntents::DIRECT_MESSAGES
        | GatewayIntents::MESSAGE_CONTENT;

    serenity::Client::builder(discord_token, intents)
        .event_handler(Bot {
            gpt_token: gpt_token.to_owned(),
            client: reqwest::Client::new(),
        })
        .await
        .expect("Err creating client")
}
