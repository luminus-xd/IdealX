use crate::Data;
use chrono::Utc;
use poise::serenity_prelude::{CreateEmbed};

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

    let embed = CreateEmbed::new()
        .title("🔄 会話リセット完了")
        .description("会話コンテキストをリセットしました。\nこれ以降のメッセージのみ AI への入力として使用します。")
        .color(0x57F287);

    ctx.send(poise::CreateReply::default().embed(embed)).await?;
    Ok(())
}
