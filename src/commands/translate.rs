use crate::{
    claude::{MessageContent, RequestMessage},
    Data,
};
use poise::serenity_prelude::CreateEmbed;
use tracing::warn;

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

// Claude APIからの翻訳レスポンスをデシリアライズする型
#[derive(Debug, serde::Deserialize)]
struct TranslationVariant {
    translation: String,
    grammar_note: String,
    example: String,
    example_translation: String,
}

#[derive(Debug, serde::Deserialize)]
struct TranslationResult {
    slang_explanation: Option<String>,
    variants: Vec<TranslationVariant>,
}

const TRANSLATE_SYSTEM_PROMPT: &str = r#"あなたは翻訳の専門家です。ユーザーから翻訳を依頼されたら、必ず以下のJSON形式のみで回答してください。JSON以外のテキスト、マークダウン記法（```など）、前置き、補足は一切不要です。

{
  "slang_explanation": null,
  "variants": [
    {
      "translation": "翻訳文1",
      "grammar_note": "文法・語法・ニュアンスの解説",
      "example": "翻訳先言語での例文",
      "example_translation": "例文の日本語訳"
    }
  ]
}

ルール:
- variantsは必ず3つ出力する（フォーマル/標準/カジュアルなど異なるニュアンスで）
- grammar_noteは言語学習者向けに、文法・語法・ニュアンスの違いを簡潔に解説する
- exampleは翻訳先言語での自然な応用例文（入力テキストをそのまま使い回さない）
- example_translationはexampleの日本語訳
- 入力がスラング・俗語・隠語・ネットスラングの場合のみ、slang_explanationにその意味・由来・使われ方を日本語で説明する。スラングでなければnullにする
- 出力はJSON以外を含めないこと"#;

static JSON_BLOCK_RE: std::sync::LazyLock<regex::Regex> =
    std::sync::LazyLock::new(|| regex::Regex::new(r"(?s)```(?:json)?\s*(.*?)\s*```").unwrap());

fn parse_translation_response(raw: &str) -> Option<TranslationResult> {
    // まずそのままJSONパースを試みる
    if let Ok(result) = serde_json::from_str::<TranslationResult>(raw.trim()) {
        return Some(result);
    }

    // 失敗したら ```json ... ``` ブロックを抽出して再試行
    let result = JSON_BLOCK_RE
        .captures(raw)
        .and_then(|c| c.get(1))
        .and_then(|m| serde_json::from_str::<TranslationResult>(m.as_str().trim()).ok());

    if result.is_none() {
        warn!("Failed to parse translation response: {}", raw);
    }

    result
}

fn truncate_chars(s: &str, max: usize) -> String {
    if max == 0 {
        return String::new();
    }
    if s.chars().count() <= max {
        s.to_string()
    } else {
        let truncated: String = s.chars().take(max - 1).collect();
        format!("{}…", truncated)
    }
}

fn base_embed(lang: &str) -> CreateEmbed {
    CreateEmbed::new()
        .title(format!("🌐 {} への翻訳", lang))
        .color(0x9B59B6)
}

fn build_translation_embed(
    lang: &str,
    input_text: &str,
    result: &TranslationResult,
) -> CreateEmbed {
    let display_text = truncate_chars(input_text, 500);

    let mut description = format!("**📝 原文**\n{}", display_text);

    if let Some(ref slang) = result.slang_explanation {
        let slang_text = truncate_chars(slang, 800);
        description.push_str(&format!("\n\n**💡 スラング解説**\n{}", slang_text));
    }

    let mut embed = base_embed(lang).description(truncate_chars(&description, 4096));

    for (i, variant) in result.variants.iter().enumerate().take(3) {
        let num = i + 1;
        let translation = truncate_chars(&variant.translation, 1024);
        let grammar = truncate_chars(&variant.grammar_note, 1000);
        let example_text = format!(
            "{}\n*{}*",
            truncate_chars(&variant.example, 480),
            truncate_chars(&variant.example_translation, 480)
        );

        embed = embed
            .field(format!("🔹 パターン {}", num), &translation, false)
            .field("📖 解説", &grammar, true)
            .field("💬 例文", truncate_chars(&example_text, 1024), true);
    }

    embed
}

fn build_fallback_embed(lang: &str, input_text: &str, raw: &str) -> CreateEmbed {
    let display_text = truncate_chars(input_text, 1000);
    let response_text = truncate_chars(raw, 1020);

    base_embed(lang)
        .field("原文", &display_text, false)
        .field(lang, &response_text, false)
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
        "以下のテキストを{}に翻訳してください:\n\n{}",
        lang, text
    );
    let request_messages = vec![RequestMessage {
        role: "user",
        content: MessageContent::Text(prompt),
    }];

    match crate::claude::get_claude_response(
        request_messages,
        &ctx.data().claude_token,
        &ctx.data().client,
        Some(TRANSLATE_SYSTEM_PROMPT),
    )
    .await
    {
        Ok(response) => {
            let embed = match parse_translation_response(&response) {
                Some(result) if !result.variants.is_empty() => {
                    build_translation_embed(lang, &text, &result)
                }
                _ => build_fallback_embed(lang, &text, &response),
            };
            ctx.send(poise::CreateReply::default().embed(embed)).await?;
        }
        Err(e) => {
            let embed = CreateEmbed::new()
                .title("❌ エラー")
                .description(format!("翻訳中にエラーが発生しました: {}", e))
                .color(0xED4245);
            ctx.send(poise::CreateReply::default().embed(embed)).await?;
        }
    }

    Ok(())
}
