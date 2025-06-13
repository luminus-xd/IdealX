mod claude;

mod commands {
    pub mod age;
}

use claude::{get_claude_response, split_message, RequestMessage};
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

use std::sync::Arc;
use tracing::{error, info};

// Poiseフレームワークのデータ型
#[derive(Clone)]
pub struct Data {
    pub claude_token: String,
    pub client: reqwest::Client,
}

#[derive(Clone)]
struct Bot {
    claude_token: String,
    client: reqwest::Client,
    target_server_ids: Arc<Vec<u64>>,
    target_forum_channel_ids: Arc<Vec<u64>>,
}

/// ユーザーかどうかを判定する関数
fn is_user(author: &User) -> bool {
    !author.bot
}

/// メッセージにBotへのメンションが含まれているかを判定する関数
fn is_inclued_bot_mention(ctx: &Context, message: &Message) -> bool {
    message
        .mentions
        .iter()
        .any(|user| user.id == ctx.cache.current_user().id)
}

/// メッセージをAPIリクエスト形式に変換する関数
fn build_json(messages: Vec<Message>) -> Vec<RequestMessage<'static>> {
    let mention_regexp = Regex::new(r"<@(\d+)>").unwrap();
    messages
        .iter()
        .rev()
        .filter_map(|message| {
            let content = mention_regexp.replace_all(&message.content, "").trim().to_string();
            
            // 空のコンテンツのメッセージは除外
            if content.is_empty() {
                info!("Skipping empty message from user: {}", message.author.name);
                return None;
            }

            let role = if is_user(&message.author) {
                "user"
            } else {
                "assistant"
            };
            Some(RequestMessage { role, content })
        })
        .collect()
}

// Bot構造体のメソッド実装
impl Bot {
    /// 特定のサーバーの特定のフォーラムチャンネルかどうかを判定するメソッド
    async fn should_auto_respond(&self, ctx: &Context, msg: &Message) -> bool {
        // サーバーIDが設定されていない場合は無効
        if self.target_server_ids.is_empty() || self.target_forum_channel_ids.is_empty() {
            return false;
        }

        // DMの場合は対象外
        let guild_id = match msg.guild_id {
            Some(id) => id,
            None => return false,
        };

        // 対象サーバーでない場合は対象外
        if !self.target_server_ids.contains(&guild_id.get()) {
            return false;
        }

        // チャンネルの情報を取得
        let channel = match msg.channel_id.to_channel(&ctx.http).await {
            Ok(channel) => channel,
            Err(e) => {
                error!("Error fetching channel: {}", e);
                return false;
            }
        };

        // フォーラム内のスレッドかどうかを確認
        match channel {
            serenity::model::channel::Channel::Guild(guild_channel) => {
                // スレッドの場合、親チャンネルを確認
                match guild_channel.kind {
                    serenity::model::channel::ChannelType::PublicThread
                    | serenity::model::channel::ChannelType::PrivateThread => {
                        // 親チャンネルIDを取得
                        if let Some(parent_id) = guild_channel.parent_id {
                            // 対象フォーラムチャンネルかどうか確認
                            self.target_forum_channel_ids.contains(&parent_id.get())
                        } else {
                            false
                        }
                    }
                    _ => false,
                }
            }
            _ => false,
        }
    }

    /// フォーラムのタイトルとディスクリプションを取得するメソッド
    async fn get_forum_info(
        &self,
        ctx: &Context,
        msg: &Message,
    ) -> (Option<String>, Option<String>) {
        // チャンネルの情報を取得
        let channel = match msg.channel_id.to_channel(&ctx.http).await {
            Ok(channel) => channel,
            Err(e) => {
                error!("Error fetching channel: {}", e);
                return (None, None);
            }
        };

        match channel {
            serenity::model::channel::Channel::Guild(guild_channel) => {
                // スレッドの場合
                match guild_channel.kind {
                    serenity::model::channel::ChannelType::PublicThread
                    | serenity::model::channel::ChannelType::PrivateThread => {
                        // スレッドのタイトル（名前）を取得
                        let title = guild_channel.name;

                        // スレッドの最初のメッセージを取得（ディスクリプションとして扱う）
                        let builder = serenity::builder::GetMessages::new().limit(1);
                        let messages = match msg.channel_id.messages(&ctx.http, builder).await {
                            Ok(messages) => messages,
                            Err(e) => {
                                error!("Error fetching first message: {}", e);
                                return (Some(title), None);
                            }
                        };

                        // 最初のメッセージがあれば、それをディスクリプションとして使用
                        let description = if !messages.is_empty() {
                            Some(messages.last().unwrap().content.clone())
                        } else {
                            None
                        };

                        (Some(title), description)
                    }
                    _ => (None, None),
                }
            }
            _ => (None, None),
        }
    }

    /// Claudeにリクエストを送信し、結果を処理するメソッド
    async fn process_claude_request(
        &self,
        ctx: &Context,
        msg: &Message,
        title: Option<&str>,
        description: Option<&str>,
    ) {
        let channel_id = msg.channel_id;

        // チャンネルの情報を取得
        let channel = match channel_id.to_channel(&ctx.http).await {
            Ok(channel) => channel,
            Err(e) => {
                error!("Error fetching channel: {}", e);
                return;
            }
        };

        // メッセージ取得の制限を設定
        // フォーラム内のスレッドの場合は全てのメッセージを取得（最大100件）
        let limit = match channel {
            serenity::model::channel::Channel::Guild(guild_channel) => {
                match guild_channel.kind {
                    serenity::model::channel::ChannelType::PublicThread
                    | serenity::model::channel::ChannelType::PrivateThread => 100, // フォーラム内のスレッドの場合は最大数を設定
                    _ => 5, // 通常のチャンネルの場合は15件
                }
            }
            _ => 5, // その他のチャンネルタイプの場合は15件
        };

        info!("Fetching {} messages from channel", limit);

        let builder = serenity::builder::GetMessages::new().limit(limit);
        let messages = match channel_id.messages(&ctx.http, builder).await {
            Ok(messages) => messages,
            Err(e) => {
                error!("Error fetching messages: {}", e);
                return;
            }
        };

        // 通常のメッセージをリクエスト形式に変換
        let mut request_body: Vec<RequestMessage> = build_json(messages);

        // タイトルとディスクリプションがある場合は、先頭に追加
        if let (Some(title_text), Some(desc_text)) = (title, description) {
            // タイトルとディスクリプションを含む追加メッセージを作成
            let forum_info = format!(
                "フォーラムタイトル: {}\nディスクリプション: {}",
                title_text, desc_text
            );
            request_body.insert(
                0,
                RequestMessage {
                    role: "user",
                    content: forum_info,
                },
            );
        }

        // メッセージが空の場合はデフォルトメッセージを追加
        if request_body.is_empty() {
            info!("No valid messages found, adding default message");
            request_body.push(RequestMessage {
                role: "user",
                content: "こんにちは".to_string(),
            });
        }

        // タイピング中の表示を開始
        let _typing = msg.channel_id.start_typing(&ctx.http);
        let claude_message =
            match get_claude_response(request_body, &self.claude_token, &self.client).await {
                Ok(text) => text,
                Err(e) => {
                    error!("Error Claude response: {}", e);
                    // エラーの詳細をユーザーに通知
                    let error_msg = format!("Claude APIエラーが発生しました: {}", e);
                    if let Err(send_err) = msg.channel_id.say(&ctx.http, &error_msg).await {
                        error!("Failed to send error message: {:?}", send_err);
                    }
                    return;
                }
            };

        // メッセージを2000文字ごとに分割
        const DISCORD_MAX_LENGTH: usize = 2000;
        let split_messages = split_message(&claude_message, DISCORD_MAX_LENGTH - 50); // メンションなどの余裕を持たせる

        // 最初のメッセージ
        let first_response = if is_inclued_bot_mention(ctx, msg) {
            // メンションされた場合はメンションを含める
            MessageBuilder::new()
                .mention(&msg.author)
                .push("\n")
                .push(&split_messages[0])
                .build()
        } else {
            // メンションされていない場合はそのまま
            split_messages[0].clone()
        };

        if let Err(why) = msg.channel_id.say(&ctx.http, &first_response).await {
            error!("Error sending first message: {:?}", why);
            return;
        }

        // 残りのメッセージを送信
        for chunk in split_messages.iter().skip(1) {
            if let Err(why) = msg.channel_id.say(&ctx.http, chunk).await {
                error!("Error sending message chunk: {:?}", why);
                break;
            }
        }
    }
}

#[async_trait]
impl EventHandler for Bot {
    async fn message(&self, ctx: Context, msg: Message) {
        // ユーザーからのメッセージのみ処理
        if !is_user(&msg.author) {
            return;
        }

        // メンションされた場合の処理（既存の機能）
        if is_inclued_bot_mention(&ctx, &msg) {
            self.process_claude_request(&ctx, &msg, None, None).await;
        }
        // 特定のサーバーの特定のフォーラムチャンネルでの処理（新機能）
        else if self.should_auto_respond(&ctx, &msg).await {
            // フォーラムのタイトルとディスクリプションを取得
            let (title, description) = self.get_forum_info(&ctx, &msg).await;
            self.process_claude_request(&ctx, &msg, title.as_deref(), description.as_deref())
                .await;
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
        info!("cache ready");
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

    // ターゲットサーバーIDとフォーラムチャンネルIDを読み込む
    let target_server_ids = if let Some(server_ids_str) = secret_store.get("TARGET_SERVER_IDS") {
        let server_ids: Vec<u64> = server_ids_str
            .split(',')
            .filter_map(|id| id.trim().parse().ok())
            .collect();
        info!("Loaded {} target server IDs", server_ids.len());
        Arc::new(server_ids)
    } else {
        Arc::new(Vec::new())
    };

    let target_forum_channel_ids =
        if let Some(forum_ids_str) = secret_store.get("TARGET_FORUM_CHANNEL_IDS") {
            let forum_ids: Vec<u64> = forum_ids_str
                .split(',')
                .filter_map(|id| id.trim().parse().ok())
                .collect();
            info!("Loaded {} target forum channel IDs", forum_ids.len());
            Arc::new(forum_ids)
        } else {
            Arc::new(Vec::new())
        };

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
            claude_token,
            client: reqwest::Client::new(),
            target_server_ids,
            target_forum_channel_ids,
        })
        .framework(framework)
        .await
        .expect("Err creating client");

    Ok(shuttle_serenity::SerenityService(client))
}
