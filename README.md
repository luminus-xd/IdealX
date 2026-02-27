# IdealX — Claude AI搭載 Discord Bot

Vercel [Chat SDK](https://chat-sdk.dev/) と [AI SDK](https://ai-sdk.dev/) で構築した Discord Bot です。
Anthropic Claude API と連携して、メンションや特定のフォーラムチャンネルで自動的にAI応答を返します。

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
- `ぬるぽ` → `ガッ`

## 必要なもの

- Node.js 22+
- Discord Bot Token（**MESSAGE CONTENT INTENT** が必要）
- Anthropic Claude API Key
- Discord Application の Public Key と Application ID

## セットアップ

```bash
# 依存関係インストール
npm install

# 環境変数の設定（.env.example をコピーして編集）
cp .env.example .env

# スラッシュコマンドの登録（初回のみ）
npm run register
```

## ローカルでの実行

```bash
# 環境変数を設定して起動（開発モード）
npm run dev

# または本番ビルド
npm run build
npm start
```

### 環境変数

| 変数名 | 説明 |
|--------|------|
| `DISCORD_BOT_TOKEN` | Discord Bot トークン |
| `DISCORD_PUBLIC_KEY` | Discord Application の Ed25519 Public Key |
| `DISCORD_APPLICATION_ID` | Discord Application ID |
| `ANTHROPIC_API_KEY` | Anthropic Claude API キー |
| `TARGET_SERVER_IDS` | 自動応答を有効にするサーバーID（カンマ区切り） |
| `TARGET_FORUM_CHANNEL_IDS` | 自動応答を有効にするフォーラムチャンネルID（カンマ区切り） |
| `PORT` | サーバーポート（デフォルト: 3000） |

## Discord Developer Portal 設定

1. **Interactions Endpoint URL** を `https://<your-domain>/api/webhooks/discord` に設定
2. **MESSAGE CONTENT INTENT** を有効化
3. Bot に必要な権限を付与

## Railway へのデプロイ

このBotはRailwayでの運用を想定しています。

1. GitHubリポジトリをRailwayに接続
2. Railwayダッシュボードで環境変数を設定
3. `git push` するだけで自動デプロイ

> [!WARNING]
> APIトークン類は絶対にGitにコミットしないでください。環境変数はRailwayダッシュボードで管理します。

## アーキテクチャ

```
Discord Gateway (WebSocket)
    ↓
Chat SDK Discord Adapter → Gateway Listener
    ↓                           ↓
    ↓                    HTTP POST (forwarded events)
    ↓                           ↓
Discord HTTP Interactions → Hono Server (/api/webhooks/discord)
                                ↓
                      bot.webhooks.discord()
                                ↓
                      Chat SDK Event Router
                                ↓
              ┌─────────────────────────────────┐
              │ onNewMention    → AI Response    │
              │ onSubscribedMsg → Forum Response │
              │ onNewMessage    → Auto / Easter  │
              │ onReaction      → 📝 Summarize  │
              │ onSlashCommand  → Commands       │
              └─────────────────────────────────┘
```

## 技術スタック

- **言語**: TypeScript (ES2022)
- **Bot フレームワーク**: Vercel Chat SDK + Discord Adapter
- **AI**: AI SDK + Anthropic Provider（claude-sonnet-4-6、ウェブ検索対応）
- **HTTP サーバー**: Hono + @hono/node-server
- **ホスティング**: Railway
