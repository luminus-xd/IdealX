mod claude;

mod commands {
    pub mod age;
}

use claude::get_claude_response;
use claude::split_message;
use claude::RequestMessage;
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

// Poiseフレームワークのデータ型
#[derive(Clone)]
pub struct Data {
    pub claude_token: String,
    pub client: reqwest::Client,
}

struct Bot {
    claude_token: String,
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
            let builder = serenity::builder::GetMessages::new().limit(15);
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
            let claude_message =
                match get_claude_response(requset_body, &self.claude_token, &self.client).await {
                    Ok(text) => text,
                    Err(e) => {
                        println!("Error Claude response: {}", e);
                        return;
                    }
                };
            
            // メッセージを2000文字ごとに分割
            const DISCORD_MAX_LENGTH: usize = 2000;
            let split_messages = split_message(&claude_message, DISCORD_MAX_LENGTH - 50); // メンションなどの余裕を持たせる
            
            // 最初のメッセージにはメンションを含める
            let first_response = MessageBuilder::new()
                .mention(&msg.author)
                .push("\n")
                .push(&split_messages[0])
                .build();
                
            if let Err(why) = msg.channel_id.say(&ctx.http, &first_response).await {
                println!("Error sending first message: {:?}", why);
                return;
            }
            
            // 残りのメッセージを送信
            for chunk in split_messages.iter().skip(1) {
                if let Err(why) = msg.channel_id.say(&ctx.http, chunk).await {
                    println!("Error sending message chunk: {:?}", why);
                    break;
                }
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
    let claude_token = secret_store
        .get("CLAUDE_TOKEN")
        .context("'CLAUDE_TOKEN' was not found")?;

    // クローンを作成して所有権の問題を回避
    let claude_token_for_framework = claude_token.clone();
    
    // Poiseフレームワークの設定
    let framework = poise::Framework::builder()
        .options(poise::FrameworkOptions {
            commands: vec![commands::age::age()],
            ..Default::default()
        })
        .setup(move |ctx, _ready, framework| {
            // moveキーワードを追加して変数の所有権をクロージャに移動
            // claude_token_for_frameworkの所有権がクロージャに移動するので、
            // 内部でのクローンは不要になります
            
            Box::pin(async move {
                // グローバルにコマンドを登録
                poise::builtins::register_globally(ctx, &framework.options().commands).await?;
                Ok(Data {
                    claude_token: claude_token_for_framework,
                    client: reqwest::Client::new(),
                })
            })
        })
        .build();

    // Serenityクライアントの設定
    let intents = GatewayIntents::GUILD_MESSAGES
        | GatewayIntents::DIRECT_MESSAGES
        | GatewayIntents::MESSAGE_CONTENT;

    let client = serenity::Client::builder(discord_token, intents)
        .event_handler(Bot {
            claude_token: claude_token,
            client: reqwest::Client::new(),
        })
        .framework(framework)
        .await
        .expect("Err creating client");

    Ok(shuttle_serenity::SerenityService(client))
}