use crate::{claude::RequestMessage, Data};

type Error = Box<dyn std::error::Error + Send + Sync>;
type Context<'a> = poise::Context<'a, Data, Error>;

#[derive(Debug, poise::ChoiceParameter)]
pub enum Language {
    #[name = "日本語"]
    Japanese,
    #[name = "英語"]
    English,
    #[name = "中国語（簡体字）"]
    ChineseSimplified,
    #[name = "中国語（繁体字）"]
    ChineseTraditional,
    #[name = "韓国語"]
    Korean,
    #[name = "フランス語"]
    French,
    #[name = "ドイツ語"]
    German,
    #[name = "スペイン語"]
    Spanish,
    #[name = "ポルトガル語"]
    Portuguese,
    #[name = "イタリア語"]
    Italian,
    #[name = "ロシア語"]
    Russian,
    #[name = "アラビア語"]
    Arabic,
}

impl Language {
    fn label(&self) -> &str {
        match self {
            Language::Japanese => "日本語",
            Language::English => "英語",
            Language::ChineseSimplified => "中国語（簡体字）",
            Language::ChineseTraditional => "中国語（繁体字）",
            Language::Korean => "韓国語",
            Language::French => "フランス語",
            Language::German => "ドイツ語",
            Language::Spanish => "スペイン語",
            Language::Portuguese => "ポルトガル語",
            Language::Italian => "イタリア語",
            Language::Russian => "ロシア語",
            Language::Arabic => "アラビア語",
        }
    }
}

/// テキストを指定言語に翻訳するコマンド
#[poise::command(slash_command, prefix_command)]
pub async fn translate(
    ctx: Context<'_>,
    #[description = "翻訳先の言語"] language: Language,
    #[description = "翻訳するテキスト"] text: String,
) -> Result<(), Error> {
    ctx.defer().await?;

    let lang = language.label();
    let prompt = format!(
        "以下のテキストを{}に翻訳してください。翻訳文のみ出力してください:\n\n{}",
        lang, text
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
            ctx.say(format!("**{}への翻訳:**\n{}", lang, response))
                .await?;
        }
        Err(e) => {
            ctx.say(format!("翻訳中にエラーが発生しました: {}", e))
                .await?;
        }
    }

    Ok(())
}
