use chrono::{Datelike, Utc};

use poise::serenity_prelude as serenity;

pub struct Data {} // User data, which is stored and accessible in all command invocations
type Error = Box<dyn std::error::Error + Send + Sync>;
type Context<'a> = poise::Context<'a, Data, Error>;

/// 選択したユーザーのDiscordに生誕した日付と経過日数を表示します
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

    let response = format!(
        "{}さんは{}年{}月{}日にDiscordに生まれました。\n{}日が経過しています。",
        u.name,
        created_at.year(),
        created_at.month(),
        created_at.day(),
        days_passed
    );

    ctx.say(response).await?;
    Ok(())
}
