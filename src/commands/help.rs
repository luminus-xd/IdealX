use crate::Data;

type Error = Box<dyn std::error::Error + Send + Sync>;
type Context<'a> = poise::Context<'a, Data, Error>;

/// IdealX Botの使い方を表示するコマンド
#[poise::command(slash_command, prefix_command)]
pub async fn help(ctx: Context<'_>) -> Result<(), Error> {
    let response = "**IdealX Bot コマンド一覧**\n\
        \n\
        **スラッシュコマンド**\n\
        `/age [ユーザー]` — DiscordアカウントID作成日と経過日数を表示\n\
        `/summarize [件数]` — 直近のメッセージをAIで要約（デフォルト10件、最大50件）\n\
        `/translate [言語] [テキスト]` — テキストを指定言語に翻訳\n\
        `/clear` — このチャンネルの会話コンテキストをリセット\n\
        `/help` — このヘルプを表示\n\
        \n\
        **メンション**\n\
        `@IdealX [メッセージ]` — AIに質問・相談（ウェブ検索対応）\n\
        \n\
        **リアクション**\n\
        📝 リアクション → メッセージを要約してチャンネルに投稿";

    ctx.say(response).await?;
    Ok(())
}
