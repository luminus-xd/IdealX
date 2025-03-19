mod chatgpt;

mod commands {
    pub mod age;
}

use chatgpt::get_gpt_response;
use chatgpt::RequestMessage;
use regex::Regex;

use poise::{serenity_prelude as serenity, serenity_prelude::ActivityData};

use anyhow::Context as _;
use serenity::async_trait;
use serenity::model::channel::Message;
use serenity::model::gateway::Ready;
use serenity::model::id::GuildId;
use serenity::model::user::OnlineStatus;
use serenity::model::user::User;
use serenity::prelude::*;
use serenity::utils::MessageBuilder;

use shuttle_runtime::SecretStore;

use tracing::{error, info};

struct Bot {
    gpt_token: String,
    client: reqwest::Client,
}

/// ユーザーかどうかを判定する関数
fn is_user(author: &User) -> bool {
    return !author.bot;
}

/// メッセージにBotへのメンションが含まれているかを判定する関数
fn is_inclued_bot_mention(ctx: &Context, message: &Message) -> bool {
    return message
        .mentions
        .iter()
        .any(|user| user.id == ctx.cache.current_user().id); // メンションされたユーザーIDがBotのIDと一致するかを判定
}

/// メッセージをAPIリクエスト形式に変換する関数
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
            let builder = serenity::builder::GetMessages::new().limit(5);
            let messages = match channel_id.messages(&ctx.http, builder).await {
                Ok(messages) => messages,
                Err(e) => {
                    println!("Error fetching messages: {}", e);
                    return;
                }
            };

            let requset_body: Vec<RequestMessage> = build_json(messages);
            // タイピング中の表示を開始
            let _typing = msg.channel_id.start_typing(&ctx.http);
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
    }

    /// Botが起動したときのイベントハンドラ
    async fn ready(&self, ctx: Context, ready: Ready) {
        info!("{} is connected!", ready.user.name);

        // Botのアクティビティを設定
        let activity = ActivityData::playing("Good Night");
        // Botのオンラインステータスを設定
        let status = OnlineStatus::Idle;

        // Botのプレゼンスを設定
        ctx.set_presence(Some(activity), status);
    }

    // Serenity 0.12では cache featureが標準で含まれるようになったため、cfg属性は不要
    async fn cache_ready(&self, _ctx: Context, _guilds: Vec<GuildId>) {
        println!("cache ready");
    }
}

#[shuttle_runtime::main]
async fn serenity(
    #[shuttle_runtime::Secrets] secret_store: SecretStore,
) -> shuttle_serenity::ShuttleSerenity {
    let discord_token = secret_store
        .get("DISCORD_TOKEN")
        .context("'DISCORD_TOKEN' was not found")?;
    let gpt_token = secret_store
        .get("CHATGPT_TOKEN")
        .context("'CHATGPT_TOKEN' was not found")?;

    let client = get_client(&discord_token, &gpt_token).await;

    // [WIP] コマンド登録のタイミングをどこかで設定する
    // register_commands(&discord_token).await;

    Ok(shuttle_serenity::SerenityService(client))
}

/// トークン情報などを設定し、クライアントを取得
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

/// コマンドを登録する非同期関数
async fn register_commands(discord_token: &str) {
    let framework = poise::Framework::builder()
        .options(poise::FrameworkOptions {
            commands: vec![commands::age::age()],
            ..Default::default()
        })
        .build();

    let mut client =
        serenity::Client::builder(discord_token, serenity::GatewayIntents::non_privileged())
            .framework(framework)
            .await
            .expect("Err creating client");

    if let Err(why) = client.start().await {
        println!("Client error: {:?}", why);
    }
}