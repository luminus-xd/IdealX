/**
 * Discord スラッシュコマンドを登録するスクリプト
 * 使用方法: npm run register
 */
import { config } from "dotenv";
config();

const APPLICATION_ID = process.env.DISCORD_APPLICATION_ID;
const BOT_TOKEN = process.env.DISCORD_BOT_TOKEN;

if (!APPLICATION_ID || !BOT_TOKEN) {
  console.error(
    "DISCORD_APPLICATION_ID と DISCORD_BOT_TOKEN を設定してください。",
  );
  process.exit(1);
}

const commands = [
  {
    name: "help",
    description: "IdealXの使い方を表示します",
  },
  {
    name: "age",
    description: "Discordアカウントの作成日と経過日数を表示します",
    options: [
      {
        name: "user",
        description: "ユーザーを選択してください",
        type: 6, // USER type
        required: false,
      },
    ],
  },
  {
    name: "summarize",
    description: "直近のメッセージをAIで要約します",
    options: [
      {
        name: "count",
        description: "要約するメッセージ数（1〜50、デフォルト: 10）",
        type: 4, // INTEGER type
        required: false,
        min_value: 1,
        max_value: 50,
      },
    ],
  },
  {
    name: "translate",
    description: "テキストを指定した言語に翻訳します",
    options: [
      {
        name: "language",
        description: "翻訳先の言語",
        type: 3, // STRING type
        required: true,
        choices: [
          { name: "日本語", value: "japanese" },
          { name: "英語", value: "english" },
          { name: "中国語（簡体字）", value: "chinese_simplified" },
          { name: "中国語（繁体字）", value: "chinese_traditional" },
          { name: "韓国語", value: "korean" },
          { name: "フランス語", value: "french" },
          { name: "ドイツ語", value: "german" },
          { name: "スペイン語", value: "spanish" },
          { name: "ポルトガル語", value: "portuguese" },
          { name: "イタリア語", value: "italian" },
          { name: "ロシア語", value: "russian" },
          { name: "アラビア語", value: "arabic" },
        ],
      },
      {
        name: "text",
        description: "翻訳するテキスト",
        type: 3, // STRING type
        required: true,
      },
    ],
  },
  {
    name: "clear",
    description: "チャンネルの会話コンテキストをリセットします",
  },
];

async function registerCommands() {
  const url = `https://discord.com/api/v10/applications/${APPLICATION_ID}/commands`;

  const response = await fetch(url, {
    method: "PUT",
    headers: {
      "Content-Type": "application/json",
      Authorization: `Bot ${BOT_TOKEN}`,
    },
    body: JSON.stringify(commands),
  });

  if (response.ok) {
    const data = await response.json();
    console.log(
      `${(data as unknown[]).length} 個のスラッシュコマンドを登録しました。`,
    );
  } else {
    const error = await response.text();
    console.error("コマンド登録に失敗しました:", response.status, error);
    process.exit(1);
  }
}

registerCommands().catch(console.error);
