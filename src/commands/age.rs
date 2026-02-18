use chrono::Utc;
use poise::serenity_prelude::{self as serenity, CreateEmbed};

use crate::Data;

type Error = Box<dyn std::error::Error + Send + Sync>;
type Context<'a> = poise::Context<'a, Data, Error>;

/// 選択したユーザーのDiscordに生誕した日付と経過日数を表示するコマンド
#[poise::command(slash_command, prefix_command)]
pub async fn age(
    ctx: Context<'_>,
    #[description = "ユーザーを選択してください"] user: Option<serenity::User>,
) -> Result<(), Error> {
    let u = user.as_ref().unwrap_or_else(|| ctx.author());

    let created_at = u.created_at().with_timezone(&Utc);
    let now = Utc::now();
    let duration = now.signed_duration_since(created_at);
    let days_passed = duration.num_days();
    let years = days_passed / 365;
    let remaining_days = days_passed % 365;

    let unix_ts = created_at.timestamp();

    let mut embed = CreateEmbed::new()
        .title(format!("🎂 {}さんのDiscordライフ", u.name))
        .color(0x3498DB)
        .field(
            "📅 アカウント作成日",
            format!("<t:{}:D>（<t:{}:R>）", unix_ts, unix_ts),
            false,
        )
        .field(
            "⏳ 経過日数",
            format!("**{}** 日（{}年 {}日）", days_passed, years, remaining_days),
            false,
        );

    if let Some(avatar_url) = u.avatar_url() {
        embed = embed.thumbnail(avatar_url);
    }

    ctx.send(poise::CreateReply::default().embed(embed)).await?;
    Ok(())
}
