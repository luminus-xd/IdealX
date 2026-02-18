use crate::{claude::RequestMessage, Data};
use poise::serenity_prelude::{self as serenity, CreateEmbed};

type Error = Box<dyn std::error::Error + Send + Sync>;
type Context<'a> = poise::Context<'a, Data, Error>;

/// チャンネルの最近のメッセージをAIで要約するコマンド
#[poise::command(slash_command, prefix_command)]
pub async fn summarize(
    ctx: Context<'_>,
    #[description = "要約するメッセージ数（1〜50、デフォルト: 10）"] count: Option<u8>,
) -> Result<(), Error> {
    let count = count.unwrap_or(10).clamp(1, 50);

    ctx.defer().await?;

    let builder = serenity::builder::GetMessages::new().limit(count);
    let messages = ctx.channel_id().messages(ctx.http(), builder).await?;

    let formatted: String = messages
        .iter()
        .rev()
        .filter(|m| !m.content.is_empty() && !m.author.bot)
        .map(|m| format!("{}: {}", m.author.name, m.content))
        .collect::<Vec<_>>()
        .join("\n");

    if formatted.is_empty() {
        let embed = CreateEmbed::new()
            .title("📝 要約")
            .description("要約するメッセージがありません。")
            .color(0xFEE75C);
        ctx.send(poise::CreateReply::default().embed(embed)).await?;
        return Ok(());
    }

    let prompt = format!("以下の会話を簡潔に要約してください:\n\n{}", formatted);
    let request_messages = vec![RequestMessage {
        role: "user",
        content: prompt,
    }];

    match crate::claude::get_claude_response(
        request_messages,
        &ctx.data().claude_token,
        &ctx.data().client,
        None,
    )
    .await
    {
        Ok(response) => {
            let embed = CreateEmbed::new()
                .title("📝 会話の要約")
                .description(&response)
                .color(0x57F287)
                .footer(serenity::CreateEmbedFooter::new(format!(
                    "直近 {} 件のメッセージより",
                    count
                )))
                .timestamp(serenity::Timestamp::now());
            ctx.send(poise::CreateReply::default().embed(embed)).await?;
        }
        Err(e) => {
            let embed = CreateEmbed::new()
                .title("❌ エラー")
                .description(format!("要約中にエラーが発生しました: {}", e))
                .color(0xED4245);
            ctx.send(poise::CreateReply::default().embed(embed)).await?;
        }
    }

    Ok(())
}
