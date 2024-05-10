use chrono::{Datelike, Utc};
use poise::serenity_prelude as serenity;

pub struct Data {}

type Error = Box<dyn std::error::Error + Send + Sync>;
type Context<'a> = poise::Context<'a, Data, Error>;

/// 選択したユーザーのDiscordに生誕した日付と経過日数を表示するコマンド
#[poise::command(slash_command, prefix_command)]
pub async fn age(
    ctx: Context<'_>, // コマンドのコンテキスト
    #[description = "ユーザーを選択してください"] user: Option<serenity::User>,
) -> Result<(), Error> {
    // 選択されたユーザーがいない場合は、コマンドの実行者を使用
    let u = user.as_ref().unwrap_or_else(|| ctx.author());

    let created_at = u.created_at().with_timezone(&Utc); // ユーザーのアカウント作成日時をUTCタイムゾーンで取得
    let now = Utc::now(); // 現在の日時をUTCタイムゾーンで取得
    let duration = now.signed_duration_since(created_at); // アカウント作成日時から現在までの期間を計算
    let days_passed = duration.num_days(); // 経過日数を計算

    let response = format!(
        "{}さんは{}年{}月{}日にDiscordに生まれました。\n{}日が経過しています。",
        u.name,             // ユーザー名
        created_at.year(),  // アカウント作成年
        created_at.month(), // アカウント作成月
        created_at.day(),   // アカウント作成日
        days_passed         // 経過日数
    );

    ctx.say(response).await?; // 送信
    Ok(())
}
