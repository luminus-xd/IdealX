mod claude;

mod commands {
    pub mod age;
    pub mod clear;
    pub mod help;
    pub mod summarize;
    pub mod translate;
}

use claude::{get_claude_response, split_message, ContentBlock, MessageContent, RequestMessage};
use regex::Regex;
use std::collections::HashMap;
use std::sync::LazyLock;

/// 静的にコンパイルされた正規表現（毎回のコンパイルを回避）
static MENTION_RE: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"<@(\d+)>").unwrap());
static SCRIPT_RE: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"(?s)<(script|style)[^>]*>.*?</(script|style)>").unwrap());
static TAG_RE: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"<[^>]+>").unwrap());
static WS_RE: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"\s+").unwrap());
static URL_RE: LazyLock<Regex> = LazyLock::new(|| Regex::new(r#"https?://[^\s<>"]+"#).unwrap());

use poise::{serenity_prelude as serenity, serenity_prelude::ActivityData};

use serenity::async_trait;
use serenity::builder::{CreateEmbed, CreateEmbedFooter, CreateMessage};
use serenity::model::channel::{Message, Reaction, ReactionType};
use serenity::model::gateway::Ready;
use serenity::model::id::{ChannelId, GuildId};
use serenity::model::user::OnlineStatus;
use serenity::model::user::User;
use serenity::prelude::*;

use std::env;
use std::sync::Arc;
use tokio::sync::RwLock;
use tracing::{error, info};
use tracing_subscriber;

/// チャンネルごとの会話リセット時刻を管理する型
pub type ResetTimes = Arc<RwLock<HashMap<ChannelId, chrono::DateTime<chrono::Utc>>>>;

// Poiseフレームワークのデータ型
#[derive(Clone)]
pub struct Data {
    pub claude_token: String,
    pub client: reqwest::Client,
    pub reset_times: ResetTimes,
}

#[derive(Clone)]
struct Bot {
    claude_token: String,
    client: reqwest::Client,
    target_server_ids: Arc<Vec<u64>>,
    target_forum_channel_ids: Arc<Vec<u64>>,
    reset_times: ResetTimes,
}

/// 添付ファイルが画像かどうかを判定する関数
fn is_image_attachment(attachment: &serenity::model::channel::Attachment) -> bool {
    attachment
        .content_type
        .as_ref()
        .map(|ct| ct.starts_with("image/"))
        .unwrap_or_else(|| attachment.height.is_some())
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

/// URLのコンテンツを取得してテキストとして返す関数
async fn fetch_url_content(client: &reqwest::Client, url: &str) -> Option<String> {
    let response = client
        .get(url)
        .timeout(std::time::Duration::from_secs(10))
        .send()
        .await
        .ok()?;

    // text/html 以外（画像・PDFなど）はスキップ
    let content_type = response
        .headers()
        .get("content-type")
        .and_then(|v| v.to_str().ok())
        .unwrap_or("")
        .to_string();
    if !content_type.contains("text/") {
        return None;
    }

    let body = response.text().await.ok()?;

    // scriptとstyleタグを内容ごと削除
    let body = SCRIPT_RE.replace_all(&body, "").to_string();

    // HTMLタグを除去
    let text = TAG_RE.replace_all(&body, " ").to_string();

    // 主要なHTMLエンティティをデコード
    let text = text
        .replace("&amp;", "&")
        .replace("&lt;", "<")
        .replace("&gt;", ">")
        .replace("&quot;", "\"")
        .replace("&nbsp;", " ");

    // 連続する空白・改行を整理
    let text = WS_RE.replace_all(&text, " ").trim().to_string();

    // 1URL あたり最大2000文字に制限
    let text: String = text.chars().take(2000).collect();

    if text.is_empty() {
        None
    } else {
        Some(text)
    }
}

/// XML構造を壊さないようにエスケープする関数
fn escape_xml(s: &str) -> String {
    s.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
}

/// メッセージをXML構造化されたAPIリクエスト形式に変換する関数
fn build_xml_messages(messages: Vec<Message>) -> Vec<RequestMessage<'static>> {
    let reversed: Vec<&Message> = messages.iter().rev().collect();

    // 時系列順に変換し、空メッセージを除外（画像のみのメッセージは保持）
    // orig_idx で元メッセージへの参照を保持し、画像URLは最後のユーザーメッセージでのみ収集する
    let parsed: Vec<(usize, bool, String, String, String)> = reversed
        .iter()
        .enumerate()
        .filter_map(|(orig_idx, message)| {
            let content = MENTION_RE
                .replace_all(&message.content, "")
                .trim()
                .to_string();

            let has_images = message.attachments.iter().any(is_image_attachment);

            if content.is_empty() && !has_images {
                info!("Skipping empty message from user: {}", message.author.name);
                return None;
            }

            let is_user_msg = is_user(&message.author);
            let author_name = message.author.name.clone();
            let timestamp = message.timestamp.to_string();
            Some((orig_idx, is_user_msg, author_name, timestamp, content))
        })
        .collect();

    // 最後のユーザーメッセージのインデックスを見つける
    let last_user_idx = parsed
        .iter()
        .rposition(|(_, is_user_msg, _, _, _)| *is_user_msg);

    if last_user_idx.is_none() {
        return Vec::new();
    }

    parsed
        .into_iter()
        .enumerate()
        .map(|(i, (orig_idx, is_user_msg, author_name, timestamp, content))| {
            if is_user_msg {
                if Some(i) == last_user_idx {
                    // 最後のユーザーメッセージ
                    let xml_content = format!(
                        "<current_message>\n<author>{}</author>\n<text>{}</text>\n</current_message>",
                        escape_xml(&author_name),
                        escape_xml(&content)
                    );

                    // 画像URLは最後のユーザーメッセージでのみ収集（最大5枚）
                    let image_urls: Vec<String> = reversed[orig_idx]
                        .attachments
                        .iter()
                        .filter(|a| is_image_attachment(a))
                        .take(5)
                        .map(|a| a.url.clone())
                        .collect();

                    if image_urls.is_empty() {
                        RequestMessage {
                            role: "user",
                            content: MessageContent::Text(xml_content),
                        }
                    } else {
                        let mut blocks: Vec<ContentBlock> = image_urls
                            .into_iter()
                            .map(ContentBlock::ImageUrl)
                            .collect();
                        blocks.push(ContentBlock::Text(xml_content));
                        RequestMessage {
                            role: "user",
                            content: MessageContent::Blocks(blocks),
                        }
                    }
                } else {
                    // 履歴のユーザーメッセージ（画像は含めない）
                    let xml_content = format!(
                        "<context>\n<message author=\"{}\" ts=\"{}\">\n{}\n</message>\n</context>",
                        escape_xml(&author_name),
                        escape_xml(&timestamp),
                        escape_xml(&content)
                    );
                    RequestMessage {
                        role: "user",
                        content: MessageContent::Text(xml_content),
                    }
                }
            } else {
                RequestMessage {
                    role: "assistant",
                    content: MessageContent::Text(escape_xml(&content)),
                }
            }
        })
        .collect()
}

/// 連続する同一ロールのメッセージを結合してロールが交互になるようにする関数
fn ensure_alternating_roles(
    messages: Vec<RequestMessage<'static>>,
) -> Vec<RequestMessage<'static>> {
    let mut result: Vec<RequestMessage<'static>> = Vec::new();

    for msg in messages {
        if let Some(last) = result.last_mut() {
            if last.role == msg.role {
                match msg.content {
                    MessageContent::Text(ref extra) => {
                        last.content.append_text(extra);
                        continue;
                    }
                    MessageContent::Blocks(ref blocks) => {
                        // Blocks型: 前のメッセージのテキストをBlocksの先頭に統合
                        let prev_content =
                            std::mem::replace(&mut last.content, MessageContent::Text(String::new()));
                        let mut merged_blocks = Vec::new();
                        if let MessageContent::Text(prev_text) = prev_content {
                            merged_blocks.push(ContentBlock::Text(prev_text));
                        }
                        merged_blocks.extend(blocks.iter().cloned());
                        last.content = MessageContent::Blocks(merged_blocks);
                        continue;
                    }
                }
            }
        }
        result.push(msg);
    }

    result
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
            serenity::model::channel::Channel::Guild(guild_channel) => match guild_channel.kind {
                serenity::model::channel::ChannelType::PublicThread
                | serenity::model::channel::ChannelType::PrivateThread => {
                    if let Some(parent_id) = guild_channel.parent_id {
                        self.target_forum_channel_ids.contains(&parent_id.get())
                    } else {
                        false
                    }
                }
                _ => false,
            },
            _ => false,
        }
    }

    /// フォーラムのタイトルとディスクリプションを取得するメソッド
    async fn get_forum_info(
        &self,
        ctx: &Context,
        msg: &Message,
    ) -> (Option<String>, Option<String>) {
        let channel = match msg.channel_id.to_channel(&ctx.http).await {
            Ok(channel) => channel,
            Err(e) => {
                error!("Error fetching channel: {}", e);
                return (None, None);
            }
        };

        match channel {
            serenity::model::channel::Channel::Guild(guild_channel) => match guild_channel.kind {
                serenity::model::channel::ChannelType::PublicThread
                | serenity::model::channel::ChannelType::PrivateThread => {
                    let title = guild_channel.name;

                    // フォーラムスレッドではチャンネルIDと最初のメッセージのIDが一致する
                    let first_message_id =
                        serenity::model::id::MessageId::new(msg.channel_id.get());
                    let description =
                        match msg.channel_id.message(&ctx.http, first_message_id).await {
                            Ok(first_msg) => {
                                if first_msg.content.is_empty() {
                                    None
                                } else {
                                    Some(first_msg.content)
                                }
                            }
                            Err(e) => {
                                error!("Error fetching first message: {}", e);
                                None
                            }
                        };

                    (Some(title), description)
                }
                _ => (None, None),
            },
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
        let limit = match channel {
            serenity::model::channel::Channel::Guild(guild_channel) => match guild_channel.kind {
                serenity::model::channel::ChannelType::PublicThread
                | serenity::model::channel::ChannelType::PrivateThread => 30,
                _ => 5,
            },
            _ => 5,
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

        // リセット時刻を確認してメッセージをフィルタリング
        let reset_time = {
            let reset_times = self.reset_times.read().await;
            reset_times.get(&channel_id).copied()
        };

        let filtered_messages: Vec<Message> = if let Some(reset_at) = reset_time {
            messages
                .into_iter()
                .filter(|m| m.timestamp.unix_timestamp() > reset_at.timestamp())
                .collect()
        } else {
            messages
        };

        // 通常のメッセージをリクエスト形式に変換
        let mut request_body: Vec<RequestMessage> = build_xml_messages(filtered_messages);

        // タイトルとディスクリプションがある場合は、XML形式で先頭に追加
        if let (Some(title_text), Some(desc_text)) = (title, description) {
            let forum_context = format!(
                "<channel_context>\n<title>{}</title>\n<description>{}</description>\n</channel_context>",
                escape_xml(title_text), escape_xml(desc_text)
            );
            request_body.insert(
                0,
                RequestMessage {
                    role: "assistant",
                    content: MessageContent::Text("コンテキストを確認しました。".to_string()),
                },
            );
            request_body.insert(
                0,
                RequestMessage {
                    role: "user",
                    content: MessageContent::Text(forum_context),
                },
            );
        }

        // ロールが交互になるように連続する同一ロールのメッセージを結合
        request_body = ensure_alternating_roles(request_body);

        // 先頭が assistant ロールの場合は除去（Claude APIは最初のメッセージがuserであることを要求）
        if request_body
            .first()
            .map(|m| m.role == "assistant")
            .unwrap_or(false)
        {
            request_body.remove(0);
        }

        // メッセージが空の場合はデフォルトメッセージを追加
        if request_body.is_empty() {
            info!("No valid user message found, adding default message");
            request_body.push(RequestMessage {
                role: "user",
                content: MessageContent::Text("こんにちは".to_string()),
            });
        }

        // システムプロンプトの定義
        const SYSTEM_PROMPT: &str = include_str!("../system_prompt.md");

        // タイピング中の表示を開始
        let _typing = msg.channel_id.start_typing(&ctx.http);
        let claude_message = match get_claude_response(
            request_body,
            &self.claude_token,
            &self.client,
            Some(SYSTEM_PROMPT),
        )
        .await
        {
            Ok(text) => text,
            Err(e) => {
                error!("Error Claude response: {}", e);
                let error_embed = CreateEmbed::new()
                    .title("❌ エラー")
                    .description(format!("Claude APIエラーが発生しました: {}", e))
                    .color(0xED4245);
                if let Err(send_err) = msg
                    .channel_id
                    .send_message(&ctx.http, CreateMessage::new().embed(error_embed))
                    .await
                {
                    error!("Failed to send error message: {:?}", send_err);
                }
                return;
            }
        };

        // 最初のチャンクをEmbedで送信（最大4000文字）、超過分はプレーンテキストで続送
        const EMBED_MAX_CHARS: usize = 4000;
        const PLAIN_MAX_LENGTH: usize = 1950;

        let (embed_text, overflow_text) = if claude_message.chars().count() <= EMBED_MAX_CHARS {
            (claude_message.clone(), String::new())
        } else {
            let split_at = claude_message
                .char_indices()
                .nth(EMBED_MAX_CHARS)
                .map(|(i, _)| i)
                .unwrap_or(claude_message.len());
            (
                claude_message[..split_at].to_string(),
                claude_message[split_at..].to_string(),
            )
        };

        let embed = CreateEmbed::new().description(&embed_text).color(0x5865F2);

        let mut create_msg = CreateMessage::new().embed(embed);
        if is_inclued_bot_mention(ctx, msg) {
            create_msg = create_msg.content(format!("<@{}>", msg.author.id));
        }

        if let Err(why) = msg.channel_id.send_message(&ctx.http, create_msg).await {
            error!("Error sending message: {:?}", why);
            return;
        }

        if !overflow_text.is_empty() {
            let remaining_chunks = split_message(&overflow_text, PLAIN_MAX_LENGTH);
            for chunk in &remaining_chunks {
                if let Err(why) = msg.channel_id.say(&ctx.http, chunk).await {
                    error!("Error sending message chunk: {:?}", why);
                    break;
                }
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

        // メンションされた場合の処理
        if is_inclued_bot_mention(&ctx, &msg) {
            self.process_claude_request(&ctx, &msg, None, None).await;
        }
        // 特定のサーバーの特定のフォーラムチャンネルでの処理
        else if self.should_auto_respond(&ctx, &msg).await {
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

    /// 📝 リアクションが追加されたときにメッセージを要約する
    async fn reaction_add(&self, ctx: Context, add_reaction: Reaction) {
        // 📝 リアクションのみ処理
        let is_memo_reaction = match &add_reaction.emoji {
            ReactionType::Unicode(s) => s == "📝",
            _ => false,
        };
        if !is_memo_reaction {
            return;
        }

        // ボットのリアクションは無視
        if let Some(user_id) = add_reaction.user_id {
            if let Ok(user) = user_id.to_user(&ctx.http).await {
                if user.bot {
                    return;
                }
            }
        }

        // リアクションされたメッセージを取得
        let message = match add_reaction.message(&ctx.http).await {
            Ok(msg) => msg,
            Err(e) => {
                error!("Error fetching reacted message: {}", e);
                return;
            }
        };

        // メッセージ内のURLを抽出（最大3件）
        let urls: Vec<&str> = URL_RE
            .find_iter(&message.content)
            .map(|m| m.as_str())
            .take(3)
            .collect();

        // テキストもURLも空なら処理しない
        if message.content.is_empty() && urls.is_empty() {
            return;
        }

        let preview: String = message.content.chars().take(50).collect();
        info!(
            "📝 reaction received, summarizing message: {} (urls: {})",
            preview,
            urls.len()
        );

        // URLのコンテンツを並列取得
        let mut url_contents: Vec<String> = Vec::new();
        for url in &urls {
            if let Some(content) = fetch_url_content(&self.client, url).await {
                url_contents.push(format!("【URL: {}】\n{}", url, content));
            }
        }

        const SYSTEM_PROMPT: &str = include_str!("../system_prompt.md");
        let mut prompt = format!(
            "以下のメッセージを簡潔に要約または説明してください:\n\n{}",
            message.content
        );
        if !url_contents.is_empty() {
            prompt.push_str(&format!(
                "\n\n--- リンク先の内容 ---\n{}",
                url_contents.join("\n\n")
            ));
        }

        let request_messages = vec![RequestMessage {
            role: "user",
            content: MessageContent::Text(prompt),
        }];

        let _typing = add_reaction.channel_id.start_typing(&ctx.http);

        match get_claude_response(
            request_messages,
            &self.claude_token,
            &self.client,
            Some(SYSTEM_PROMPT),
        )
        .await
        {
            Ok(response) => {
                let author_name = message.author.name.clone();
                let embed = CreateEmbed::new()
                    .title("📝 メッセージ要約")
                    .description(&response)
                    .color(0xFEE75C)
                    .footer(CreateEmbedFooter::new(format!(
                        "{} のメッセージより",
                        author_name
                    )))
                    .timestamp(serenity::Timestamp::now());
                if let Err(e) = add_reaction
                    .channel_id
                    .send_message(&ctx.http, CreateMessage::new().embed(embed))
                    .await
                {
                    error!("Error sending reaction response: {}", e);
                }
            }
            Err(e) => {
                error!("Error getting Claude response for reaction: {}", e);
                let error_embed = CreateEmbed::new()
                    .title("❌ エラー")
                    .description(format!("要約中にエラーが発生しました: {}", e))
                    .color(0xED4245);
                if let Err(send_err) = add_reaction
                    .channel_id
                    .send_message(&ctx.http, CreateMessage::new().embed(error_embed))
                    .await
                {
                    error!("Failed to send error message: {:?}", send_err);
                }
            }
        }
    }

    /// Botが起動したときのイベントハンドラ
    async fn ready(&self, ctx: Context, ready: Ready) {
        info!("{} is connected!", ready.user.name);

        let activity = ActivityData::playing("Good Night");
        let status = OnlineStatus::Idle;
        ctx.set_presence(Some(activity), status);
    }

    async fn cache_ready(&self, _ctx: Context, _guilds: Vec<GuildId>) {
        info!("cache ready");
    }
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt::init();

    info!("Starting IdealX Discord Bot...");
    info!("Loading environment variables...");

    info!("Available environment variables:");
    for (key, value) in env::vars() {
        if key.contains("DISCORD") || key.contains("CLAUDE") || key.contains("TARGET") {
            info!("  {}: {} characters", key, value.len());
        }
    }

    let discord_token = match env::var("DISCORD_TOKEN") {
        Ok(token) => {
            info!("DISCORD_TOKEN found (length: {})", token.len());
            if token.is_empty() {
                error!("DISCORD_TOKEN is empty!");
                return Err(anyhow::anyhow!("DISCORD_TOKEN is empty"));
            }
            token
        }
        Err(e) => {
            error!("DISCORD_TOKEN not found: {}", e);
            return Err(anyhow::anyhow!(
                "DISCORD_TOKEN environment variable was not found: {}",
                e
            ));
        }
    };

    let claude_token = match env::var("CLAUDE_TOKEN") {
        Ok(token) => {
            info!("CLAUDE_TOKEN found (length: {})", token.len());
            if token.is_empty() {
                error!("CLAUDE_TOKEN is empty!");
                return Err(anyhow::anyhow!("CLAUDE_TOKEN is empty"));
            }
            token
        }
        Err(e) => {
            error!("CLAUDE_TOKEN not found: {}", e);
            return Err(anyhow::anyhow!(
                "CLAUDE_TOKEN environment variable was not found: {}",
                e
            ));
        }
    };

    info!("Environment variables loaded successfully");

    info!("Loading target server configuration...");
    let target_server_ids = if let Ok(server_ids_str) = env::var("TARGET_SERVER_IDS") {
        let server_ids: Vec<u64> = server_ids_str
            .split(',')
            .filter_map(|id| id.trim().parse().ok())
            .collect();
        info!(
            "Loaded {} target server IDs: {:?}",
            server_ids.len(),
            server_ids
        );
        Arc::new(server_ids)
    } else {
        info!("No TARGET_SERVER_IDS found, using empty list");
        Arc::new(Vec::new())
    };

    let target_forum_channel_ids = if let Ok(forum_ids_str) = env::var("TARGET_FORUM_CHANNEL_IDS") {
        let forum_ids: Vec<u64> = forum_ids_str
            .split(',')
            .filter_map(|id| id.trim().parse().ok())
            .collect();
        info!(
            "Loaded {} target forum channel IDs: {:?}",
            forum_ids.len(),
            forum_ids
        );
        Arc::new(forum_ids)
    } else {
        info!("No TARGET_FORUM_CHANNEL_IDS found, using empty list");
        Arc::new(Vec::new())
    };

    // Bot と Poise Data で共有するリセット時刻マップ
    let reset_times: ResetTimes = Arc::new(RwLock::new(HashMap::new()));

    let claude_token_for_framework = claude_token.clone();
    let reset_times_for_framework = reset_times.clone();

    info!("Setting up Poise framework...");
    let framework = poise::Framework::builder()
        .options(poise::FrameworkOptions {
            commands: vec![
                commands::age::age(),
                commands::help::help(),
                commands::summarize::summarize(),
                commands::translate::translate(),
                commands::clear::clear(),
            ],
            ..Default::default()
        })
        .setup(move |ctx, ready, framework| {
            info!("Poise framework setup callback called");
            info!("Bot {} is connected via Poise!", ready.user.name);
            info!("Bot user ID: {}", ready.user.id);
            info!("Connected to {} guilds", ready.guilds.len());

            ctx.set_presence(
                Some(ActivityData::playing("Good Night")),
                OnlineStatus::Idle,
            );

            Box::pin(async move {
                info!("Registering commands globally...");
                match poise::builtins::register_globally(ctx, &framework.options().commands).await {
                    Ok(_) => info!("Commands registered successfully"),
                    Err(e) => error!("Failed to register commands: {:?}", e),
                }

                info!("Creating framework data...");
                Ok(Data {
                    claude_token: claude_token_for_framework,
                    client: reqwest::Client::new(),
                    reset_times: reset_times_for_framework,
                })
            })
        })
        .build();

    info!("Poise framework created successfully");

    info!("Setting up Discord client...");
    let intents = GatewayIntents::GUILD_MESSAGES
        | GatewayIntents::DIRECT_MESSAGES
        | GatewayIntents::MESSAGE_CONTENT
        | GatewayIntents::GUILD_MESSAGE_REACTIONS;

    let bot_handler = Bot {
        claude_token,
        client: reqwest::Client::new(),
        target_server_ids,
        target_forum_channel_ids,
        reset_times,
    };

    info!("Creating Discord client with bot handler and framework...");
    let mut client = match serenity::Client::builder(discord_token, intents)
        .event_handler(bot_handler)
        .framework(framework)
        .await
    {
        Ok(client) => {
            info!("Discord client created successfully with both handler and framework");
            client
        }
        Err(why) => {
            error!("Error creating client: {:?}", why);
            return Err(anyhow::anyhow!("Failed to create client: {:?}", why));
        }
    };

    info!("Starting Discord client...");

    let ctrl_c = async {
        tokio::signal::ctrl_c()
            .await
            .expect("Failed to install Ctrl+C handler");
    };

    #[cfg(unix)]
    let terminate = async {
        tokio::signal::unix::signal(tokio::signal::unix::SignalKind::terminate())
            .expect("Failed to install signal handler")
            .recv()
            .await;
    };

    #[cfg(not(unix))]
    let terminate = std::future::pending::<()>();

    tokio::select! {
        result = client.start() => {
            if let Err(why) = result {
                error!("Client error: {:?}", why);
                info!("Waiting before exit due to client error...");
                tokio::time::sleep(tokio::time::Duration::from_secs(30)).await;
                return Err(anyhow::anyhow!("Client failed to start: {:?}", why));
            }
        }
        _ = ctrl_c => {
            info!("Received Ctrl+C, shutting down...");
        }
        _ = terminate => {
            info!("Received terminate signal, shutting down...");
        }
    }

    info!("Bot shutdown gracefully");
    Ok(())
}
