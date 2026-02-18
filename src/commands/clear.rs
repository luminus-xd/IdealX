use crate::Data;
use chrono::Utc;

type Error = Box<dyn std::error::Error + Send + Sync>;
type Context<'a> = poise::Context<'a, Data, Error>;

/// このチャンネルの会話コンテキストをリセットするコマンド
#[poise::command(slash_command, prefix_command)]
pub async fn clear(ctx: Context<'_>) -> Result<(), Error> {
    let channel_id = ctx.channel_id();
    let now = Utc::now();

    {
        let mut reset_times = ctx.data().reset_times.write().await;
        reset_times.insert(channel_id, now);
    }

    ctx.say("会話コンテキストをリセットしました。これ以降のメッセージのみAIへの入力として使用します。")
        .await?;
    Ok(())
}
