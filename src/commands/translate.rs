use crate::{claude::RequestMessage, Data};

type Error = Box<dyn std::error::Error + Send + Sync>;
type Context<'a> = poise::Context<'a, Data, Error>;

/// テキストを指定言語に翻訳するコマンド
#[poise::command(slash_command, prefix_command)]
pub async fn translate(
    ctx: Context<'_>,
    #[description = "翻訳先の言語（例: 英語、中国語、フランス語）"] language: String,
    #[description = "翻訳するテキスト"] text: String,
) -> Result<(), Error> {
    ctx.defer().await?;

    let prompt = format!(
        "以下のテキストを{}に翻訳してください。翻訳文のみ出力してください:\n\n{}",
        language, text
    );
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
            ctx.say(format!("**{}への翻訳:**\n{}", language, response))
                .await?;
        }
        Err(e) => {
            ctx.say(format!("翻訳中にエラーが発生しました: {}", e))
                .await?;
        }
    }

    Ok(())
}
