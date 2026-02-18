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
| `/clear` | チャンネルの会話コンテキストをリセット |

**その他**
- X（旧Twitter）のURLを自動でvxtwitter.comに変換
- `ぬるぽ` → `ガッ`

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
