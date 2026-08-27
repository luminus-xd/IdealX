use crate::Data;
use poise::serenity_prelude::{self as serenity, CreateEmbed};

type Error = Box<dyn std::error::Error + Send + Sync>;
type Context<'a> = poise::Context<'a, Data, Error>;

/// IdealX Botの使い方を表示するコマンド
#[poise::command(slash_command, prefix_command)]
pub async fn help(ctx: Context<'_>) -> Result<(), Error> {
    let embed = CreateEmbed::new()
        .title("🤖 IdealX Bot")
        .description("Anthropic Claude AI を搭載した Discord ボットです。")
        .color(0x5865F2)
        .field(
            "💬 メンション",
            "`@IdealX [メッセージ]` — AIに質問・相談（ウェブ検索対応）",
            false,
        )
        .field(
            "📋 スラッシュコマンド",
            "`/age [ユーザー]` — アカウント作成日と経過日数を表示\n\
             `/summarize [件数]` — 直近のメッセージをAIで要約（最大50件）\n\
             `/translate [言語] [テキスト]` — テキストを指定言語に翻訳\n\
             `/nightlord [公開]` — プルダウンで夜ボスを選んで夜の王を逆引き\n\
             `/nightslot [dlc] [重複]` — 2〜3人を選び、キャラをスロットで割り当て\n\
             `/clear` — このチャンネルの会話コンテキストをリセット\n\
             `/overlay` — OBS配信用コメントオーバーレイを操作\n\
             `/help` — このヘルプを表示",
            false,
        )
        .field(
            "⚡ リアクション",
            "📝 リアクション → メッセージを要約してチャンネルに投稿",
            false,
        )
        .footer(serenity::CreateEmbedFooter::new(
            "Powered by Claude claude-sonnet-4-6",
        ));

    ctx.send(poise::CreateReply::default().embed(embed)).await?;
    Ok(())
}
