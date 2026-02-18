use crate::{claude::RequestMessage, Data};
use poise::serenity_prelude as serenity;

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
        ctx.say("要約するメッセージがありません。").await?;
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
            ctx.say(format!(
                "**直近{}件のメッセージの要約:**\n{}",
                count, response
            ))
            .await?;
        }
        Err(e) => {
            ctx.say(format!("要約中にエラーが発生しました: {}", e))
                .await?;
        }
    }

    Ok(())
}
