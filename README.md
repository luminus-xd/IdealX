# IdealX — Claude AI搭載 Discord Bot

RustとSerenityで作ったDiscord Botです。  
Anthropic Claude APIと連携して、メンションや特定のフォーラムチャンネルで自動的にAI応答を返します。

## 機能

**AI応答**
- メンションすると直近の会話コンテキストを読み取り、Claude（claude-sonnet-4-6）が回答
- フォーラムチャンネルでの投稿にはメンションなしで自動応答
- ウェブ検索ツール連携で最新情報も参照可能
- 2000文字を超えるレスポンスは自動で分割送信

**リアクション機能**
- メッセージに 📝 リアクションをつけると、そのメッセージをAIが要約してチャンネルに投稿

**スラッシュコマンド**

| コマンド | 説明 |
|----------|------|
| `/help` | コマンド一覧を表示 |
| `/age [ユーザー]` | DiscordアカウントのID作成日と経過日数を表示 |
| `/summarize [件数]` | 直近のメッセージをAIで要約（デフォルト10件、最大50件） |
| `/translate [言語] [テキスト]` | テキストを指定言語に翻訳 |
| `/nightlord [1日目/2日目]` | 片方または両方の夜ボスから夜の王候補を逆引き |
| `/clear` | チャンネルの会話コンテキストをリセット |
| `/overlay start` | このチャンネルで配信コメント表示を開始 |
| `/overlay status` | セッション状態を確認 |
| `/overlay pause` / `resume` | 表示を一時停止・再開 |
| `/overlay clear` | 表示中コメントを全消去 |
| `/overlay test` | OBSへテストコメントを表示 |
| `/overlay rotate` | OBS表示URLをローテーション |
| `/overlay end` | セッションを終了してURLを失効 |

**その他**
- `ぬるぽ` → `ガッ`

**OBSコメントオーバーレイ**
- 指定したDiscordテキストチャンネルまたはVC内チャットの新着コメントをOBS Browser Sourceへリアルタイム表示
- 最大3件、10 / 20 / 30 / 60秒表示、Dark / Light / Compactテーマ
- 配信管理者の📌リアクションでコメントを8秒間ピックアップ表示
- ✌️・🤟リアクションと、`www`・`GG`・`草`を含む投稿で画面演出
- Bot・Webhook・添付のみ投稿を除外し、URL・制御文字・Bidi文字を安全化
- 編集・削除・一括削除、WebSocket再接続、URLローテーションに対応

## 必要なもの

- Rust（stable）
- Cargo
- Discord Bot Token（**MESSAGE CONTENT INTENT** が必要）
- Anthropic Claude API Token

## ローカルでの実行

```bash
# ビルド
cargo build

# 環境変数を設定して起動
export DISCORD_TOKEN="your_discord_token"
export CLAUDE_TOKEN="your_claude_token"
export TARGET_SERVER_IDS="server_id1,server_id2"        # 自動応答を有効にするサーバーID
export TARGET_FORUM_CHANNEL_IDS="channel_id1,channel_id2"  # 自動応答を有効にするフォーラムチャンネルID

# OBSコメントオーバーレイ（利用する場合）
export OVERLAY_ENABLED="true"
export OVERLAY_PUBLIC_BASE_URL="https://your-service.example.com"
export OVERLAY_ALLOWED_OWNER_IDS="discord_user_id1,discord_user_id2"
export OVERLAY_DEFAULT_DISPLAY_SECONDS="20"
export OVERLAY_SESSION_TTL_MINUTES="480"
export PORT="3000"
cargo run
```

## Railway へのデプロイ

このBotはRailwayでの運用を想定しています。

1. GitHubリポジトリをRailwayに接続
2. Railwayダッシュボードで以下の環境変数を設定
   - `DISCORD_TOKEN`
   - `CLAUDE_TOKEN`
   - `TARGET_SERVER_IDS`（カンマ区切り）
   - `TARGET_FORUM_CHANNEL_IDS`（カンマ区切り）
   - `OVERLAY_ENABLED=true`（オーバーレイを利用する場合）
   - `OVERLAY_PUBLIC_BASE_URL`（RailwayのHTTPS公開オリジン）
   - `OVERLAY_ALLOWED_OWNER_IDS`（操作を許可するDiscordユーザーID、カンマ区切り）
   - `OVERLAY_DEFAULT_DISPLAY_SECONDS`（任意、10 / 20 / 30 / 60、既定20）
   - `OVERLAY_SESSION_TTL_MINUTES`（任意、既定480）
3. `git push` するだけで自動デプロイ

> [!WARNING]
> APIトークン類は絶対にGitにコミットしないでください。環境変数はRailwayダッシュボードで管理します。

## 開発

```bash
# フォーマット
cargo fmt

# Lint
cargo clippy
```

## 技術スタック

- **言語**: Rust (edition 2021)
- **Discordフレームワーク**: Serenity + Poise
- **AI**: Anthropic Claude API（claude-sonnet-4-6）
- **非同期ランタイム**: Tokio
- **ホスティング**: Railway
