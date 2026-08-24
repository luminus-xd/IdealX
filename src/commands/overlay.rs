use std::time::Duration;

use poise::serenity_prelude::{self as serenity, CreateEmbed, CreateMessage};

use crate::{
    overlay::{
        hub::HubError,
        model::{NewComment, OverlayTheme, SessionStatus, StartSession},
    },
    Data,
};

type Error = Box<dyn std::error::Error + Send + Sync>;
type Context<'a> = poise::Context<'a, Data, Error>;

#[derive(Clone, Copy, Debug, poise::ChoiceParameter)]
pub enum ThemeChoice {
    #[name = "Dark"]
    Dark,
    #[name = "Light"]
    Light,
    #[name = "Compact"]
    Compact,
}

impl ThemeChoice {
    fn model(self) -> OverlayTheme {
        match self {
            Self::Dark => OverlayTheme::Dark,
            Self::Light => OverlayTheme::Light,
            Self::Compact => OverlayTheme::Compact,
        }
    }
}

#[derive(Clone, Copy, Debug, poise::ChoiceParameter)]
pub enum DisplayDuration {
    #[name = "10秒"]
    Seconds10,
    #[name = "20秒"]
    Seconds20,
    #[name = "30秒"]
    Seconds30,
    #[name = "60秒"]
    Seconds60,
}

impl DisplayDuration {
    fn seconds(self) -> u64 {
        match self {
            Self::Seconds10 => 10,
            Self::Seconds20 => 20,
            Self::Seconds30 => 30,
            Self::Seconds60 => 60,
        }
    }
}

/// 配信用コメントオーバーレイを操作します
#[poise::command(
    slash_command,
    guild_only,
    subcommands("start", "status", "pause", "resume", "clear", "test", "end", "rotate")
)]
pub async fn overlay(_ctx: Context<'_>) -> Result<(), Error> {
    Ok(())
}

/// このテキストチャンネルでオーバーレイを開始します
#[poise::command(slash_command, guild_only)]
pub async fn start(
    ctx: Context<'_>,
    #[description = "表示テーマ（既定: Dark）"] theme: Option<ThemeChoice>,
    #[description = "コメントの表示時間"] display_time: Option<DisplayDuration>,
    #[description = "アバターを表示するか（既定: true）"] show_avatar: Option<bool>,
) -> Result<(), Error> {
    ctx.defer_ephemeral().await?;
    let Some((guild_id, channel_id, owner_user_id)) = authorized_scope(ctx).await? else {
        return Ok(());
    };
    if !is_regular_text_channel(ctx).await? {
        ephemeral(ctx, "通常のテキストチャンネルで実行してください。").await?;
        return Ok(());
    }

    let seconds = display_time
        .map(DisplayDuration::seconds)
        .unwrap_or(ctx.data().overlay_config.default_display_seconds);
    let request = StartSession {
        guild_id,
        channel_id,
        owner_user_id,
        theme: theme.unwrap_or(ThemeChoice::Dark).model(),
        show_avatar: show_avatar.unwrap_or(true),
        display_duration: Duration::from_secs(seconds),
    };

    let credentials = match ctx.data().overlay_hub.start(request).await {
        Ok(credentials) => credentials,
        Err(error) => {
            ephemeral(ctx, hub_error_message(error)).await?;
            return Ok(());
        }
    };

    let announcement = CreateMessage::new().embed(
        CreateEmbed::new()
            .title("📺 配信コメント表示を開始しました")
            .description("このチャンネルへの投稿は、配信映像に表示される場合があります。")
            .color(0x5865F2),
    );
    if ctx
        .channel_id()
        .send_message(ctx.http(), announcement)
        .await
        .is_err()
    {
        let _ = ctx.data().overlay_hub.end(owner_user_id, channel_id).await;
        ephemeral(
            ctx,
            "開始通知を送信できなかったため、セッションを開始しませんでした。",
        )
        .await?;
        return Ok(());
    }

    let viewer_url = viewer_url(ctx, &credentials.public_id, &credentials.capability);
    let embed = CreateEmbed::new()
        .title("OBS表示URL")
        .description(format!(
            "OBSのブラウザソースへ次のURLを貼り付けてください。URLは閲覧権限そのものなので共有しないでください。\n\n```text\n{viewer_url}\n```"
        ))
        .field("推奨サイズ", "配信キャンバスと同じサイズ（例: 1920×1080）", false)
        .color(0x57F287);
    if let Err(error) = ctx
        .send(poise::CreateReply::default().embed(embed).ephemeral(true))
        .await
    {
        let _ = ctx
            .data()
            .overlay_hub
            .end(owner_user_id, channel_id)
            .await;
        return Err(error.into());
    }
    Ok(())
}

/// 現在のオーバーレイ状態を表示します
#[poise::command(slash_command, guild_only)]
pub async fn status(ctx: Context<'_>) -> Result<(), Error> {
    let Some((guild_id, channel_id, owner_user_id)) = authorized_scope(ctx).await? else {
        return Ok(());
    };
    let Some(summary) = ctx.data().overlay_hub.status().await else {
        ephemeral(ctx, "アクティブなオーバーレイはありません。").await?;
        return Ok(());
    };
    if summary.guild_id != guild_id
        || summary.channel_id != channel_id
        || summary.owner_user_id != owner_user_id
    {
        ephemeral(ctx, "このチャンネルで操作できるオーバーレイはありません。").await?;
        return Ok(());
    }

    let status = match summary.status {
        SessionStatus::Active => "表示中",
        SessionStatus::Paused => "一時停止中",
    };
    let theme = match summary.theme {
        OverlayTheme::Dark => "Dark",
        OverlayTheme::Light => "Light",
        OverlayTheme::Compact => "Compact",
    };
    let expires_at = summary.expires_at_unix_ms / 1_000;
    let embed = CreateEmbed::new()
        .title("オーバーレイ状態")
        .field("状態", status, true)
        .field("表示中", format!("{}件", summary.comment_count), true)
        .field("テーマ", theme, true)
        .field(
            "アバター",
            if summary.show_avatar {
                "表示"
            } else {
                "非表示"
            },
            true,
        )
        .field("有効期限", format!("<t:{expires_at}:R>"), true)
        .color(0x5865F2);
    ctx.send(poise::CreateReply::default().embed(embed).ephemeral(true))
        .await?;
    Ok(())
}

/// 新着表示を停止し、現在のコメントを消去します
#[poise::command(slash_command, guild_only)]
pub async fn pause(ctx: Context<'_>) -> Result<(), Error> {
    control(ctx, ControlAction::Pause).await
}

/// 一時停止中のオーバーレイを再開します
#[poise::command(slash_command, guild_only)]
pub async fn resume(ctx: Context<'_>) -> Result<(), Error> {
    control(ctx, ControlAction::Resume).await
}

/// 現在のコメントを全消去します
#[poise::command(slash_command, guild_only)]
pub async fn clear(ctx: Context<'_>) -> Result<(), Error> {
    control(ctx, ControlAction::Clear).await
}

/// OBSへテストコメントを表示します
#[poise::command(slash_command, guild_only)]
pub async fn test(ctx: Context<'_>) -> Result<(), Error> {
    let Some((guild_id, channel_id, owner_user_id)) = authorized_scope(ctx).await? else {
        return Ok(());
    };
    if !owns_current_session(ctx, guild_id, channel_id, owner_user_id).await {
        ephemeral(ctx, "このチャンネルで操作できるオーバーレイはありません。").await?;
        return Ok(());
    }

    let message_id = u64::MAX.saturating_sub(chrono::Utc::now().timestamp_millis() as u64);
    let added = ctx
        .data()
        .overlay_hub
        .add_comment(NewComment {
            guild_id,
            channel_id,
            discord_message_id: message_id,
            author_user_id: owner_user_id,
            author_name: "IdealX テスト".to_string(),
            avatar_url: None,
            body: "コメントオーバーレイの表示テストです。".to_string(),
            created_at_unix_ms: Some(chrono::Utc::now().timestamp_millis()),
        })
        .await;
    ephemeral(
        ctx,
        if added {
            "テストコメントを送信しました。"
        } else {
            "テストコメントを表示できませんでした。一時停止状態を確認してください。"
        },
    )
    .await?;
    Ok(())
}

/// セッションを終了し、表示URLを失効させます
#[poise::command(slash_command, guild_only)]
pub async fn end(ctx: Context<'_>) -> Result<(), Error> {
    control(ctx, ControlAction::End).await
}

/// 表示URLの閲覧能力をローテーションします
#[poise::command(slash_command, guild_only)]
pub async fn rotate(ctx: Context<'_>) -> Result<(), Error> {
    let Some((_guild_id, channel_id, owner_user_id)) = authorized_scope(ctx).await? else {
        return Ok(());
    };
    match ctx
        .data()
        .overlay_hub
        .rotate(owner_user_id, channel_id)
        .await
    {
        Ok(credentials) => {
            let viewer_url = viewer_url(ctx, &credentials.public_id, &credentials.capability);
            let embed = CreateEmbed::new()
                .title("新しいOBS表示URL")
                .description(format!(
                    "旧URLは失効しました。OBSのブラウザソースを次のURLへ更新してください。\n\n```text\n{viewer_url}\n```"
                ))
                .color(0xFEE75C);
            ctx.send(poise::CreateReply::default().embed(embed).ephemeral(true))
                .await?;
        }
        Err(error) => ephemeral(ctx, hub_error_message(error)).await?,
    }
    Ok(())
}

#[derive(Clone, Copy)]
enum ControlAction {
    Pause,
    Resume,
    Clear,
    End,
}

async fn control(ctx: Context<'_>, action: ControlAction) -> Result<(), Error> {
    let Some((_guild_id, channel_id, owner_user_id)) = authorized_scope(ctx).await? else {
        return Ok(());
    };
    let result = match action {
        ControlAction::Pause => {
            ctx.data()
                .overlay_hub
                .pause(owner_user_id, channel_id)
                .await
        }
        ControlAction::Resume => {
            ctx.data()
                .overlay_hub
                .resume(owner_user_id, channel_id)
                .await
        }
        ControlAction::Clear => {
            ctx.data()
                .overlay_hub
                .clear(owner_user_id, channel_id)
                .await
        }
        ControlAction::End => ctx.data().overlay_hub.end(owner_user_id, channel_id).await,
    };
    match result {
        Ok(()) => {
            let message = match action {
                ControlAction::Pause => "オーバーレイを一時停止し、表示を消去しました。",
                ControlAction::Resume => "オーバーレイを再開しました。",
                ControlAction::Clear => "表示中のコメントを消去しました。",
                ControlAction::End => "オーバーレイを終了し、表示URLを失効させました。",
            };
            ephemeral(ctx, message).await?;
        }
        Err(error) => ephemeral(ctx, hub_error_message(error)).await?,
    }
    Ok(())
}

async fn authorized_scope(ctx: Context<'_>) -> Result<Option<(u64, u64, u64)>, Error> {
    if !ctx.data().overlay_config.enabled {
        ephemeral(ctx, "コメントオーバーレイ機能は無効です。").await?;
        return Ok(None);
    }
    let owner_user_id = ctx.author().id.get();
    if !ctx.data().overlay_config.owner_is_allowed(owner_user_id) {
        ephemeral(ctx, "このコマンドを実行する権限がありません。").await?;
        return Ok(None);
    }
    let Some(guild_id) = ctx.guild_id() else {
        ephemeral(ctx, "サーバー内のテキストチャンネルで実行してください。").await?;
        return Ok(None);
    };
    Ok(Some((
        guild_id.get(),
        ctx.channel_id().get(),
        owner_user_id,
    )))
}

async fn is_regular_text_channel(ctx: Context<'_>) -> Result<bool, Error> {
    let channel = ctx.channel_id().to_channel(ctx.http()).await?;
    Ok(matches!(
        channel,
        serenity::Channel::Guild(channel) if channel.kind == serenity::ChannelType::Text
    ))
}

async fn owns_current_session(
    ctx: Context<'_>,
    guild_id: u64,
    channel_id: u64,
    owner_user_id: u64,
) -> bool {
    ctx.data()
        .overlay_hub
        .status()
        .await
        .is_some_and(|session| {
            session.guild_id == guild_id
                && session.channel_id == channel_id
                && session.owner_user_id == owner_user_id
        })
}

fn viewer_url(ctx: Context<'_>, public_id: &str, capability: &str) -> String {
    format!(
        "{}/overlay/{}#{}",
        ctx.data().overlay_config.public_base_url,
        public_id,
        capability
    )
}

fn hub_error_message(error: HubError) -> &'static str {
    match error {
        HubError::SessionAlreadyActive => "すでに別のオーバーレイセッションが動作しています。",
        HubError::SessionNotFound => "アクティブなオーバーレイはありません。",
        HubError::Unauthorized => "このセッションを操作できるのは開始した本人だけです。",
        HubError::WrongChannel => "セッションを開始したチャンネルで実行してください。",
        HubError::AlreadyPaused => "オーバーレイはすでに一時停止しています。",
        HubError::AlreadyActive => "オーバーレイはすでに表示中です。",
    }
}

async fn ephemeral(ctx: Context<'_>, message: impl Into<String>) -> Result<(), Error> {
    ctx.send(
        poise::CreateReply::default()
            .content(message.into())
            .ephemeral(true),
    )
    .await?;
    Ok(())
}
