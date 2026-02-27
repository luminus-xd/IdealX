import { Chat, Card, CardText, Fields, Field, Divider, Section } from "chat";
import { createMemoryState } from "@chat-adapter/state-memory";
import { buildAdapters } from "./adapters.js";
import {
  streamAIResponse,
  generateSummary,
  generateTranslation,
  generateUrlSummary,
  type ConversationMessage,
} from "./ai.js";

// ========== 設定 ==========

const TARGET_SERVER_IDS = (process.env.TARGET_SERVER_IDS || "")
  .split(",")
  .filter(Boolean);
const TARGET_FORUM_CHANNEL_IDS = (process.env.TARGET_FORUM_CHANNEL_IDS || "")
  .split(",")
  .filter(Boolean);

// /clear コマンド用のリセット時刻マップ
const resetTimes = new Map<string, Date>();

// ========== Bot 初期化 ==========

const adapters = buildAdapters();
const state = createMemoryState();

export const bot = new Chat({
  userName: "IdealX",
  adapters,
  state,
});

export async function initializeBot() {
  await bot.initialize();
  console.log("IdealX bot initialized");
}

// ========== ユーティリティ関数 ==========

/** 対象フォーラムチャンネルかどうかを判定する */
function isTargetForum(threadId: unknown): boolean {
  const id = threadId as { guildId?: string; channelId?: string };
  if (!id?.guildId || !id?.channelId) return false;
  return (
    TARGET_SERVER_IDS.includes(id.guildId) &&
    TARGET_FORUM_CHANNEL_IDS.includes(id.channelId)
  );
}

/** async iterable から指定件数のメッセージを取得する */
async function collectMessages<T>(
  iterable: AsyncIterable<T>,
  limit: number,
): Promise<T[]> {
  const result: T[] = [];
  for await (const item of iterable) {
    result.push(item);
    if (result.length >= limit) break;
  }
  return result;
}

/** Chat SDK の Message 配列を ConversationMessage 形式に変換する */
function toConversationMessages(
  messages: Array<{
    text?: string;
    author?: { isMe?: boolean; isBot?: boolean | "unknown" };
  }>,
): ConversationMessage[] {
  return messages
    .filter((m) => m.text && m.text.trim())
    .map((m) => ({
      role: (m.author?.isMe ? "assistant" : "user") as "user" | "assistant",
      content: m.text!.replace(/<@!?\d+>/g, "").trim(),
    }))
    .filter((m) => m.content.length > 0);
}

/** URL からテキストコンテンツを取得する */
async function fetchUrlContent(url: string): Promise<string | null> {
  try {
    const res = await fetch(url, {
      headers: { "User-Agent": "IdealX-Bot/2.0" },
      signal: AbortSignal.timeout(10000),
    });
    const contentType = res.headers.get("content-type") || "";
    if (!contentType.includes("text/html")) return null;
    if (!res.ok) return null;

    const html = await res.text();
    const content = html
      .replace(/<script[\s\S]*?<\/script>/gi, "")
      .replace(/<style[\s\S]*?<\/style>/gi, "")
      .replace(/<[^>]+>/g, " ")
      .replace(/&amp;/g, "&")
      .replace(/&lt;/g, "<")
      .replace(/&gt;/g, ">")
      .replace(/&quot;/g, '"')
      .replace(/&#039;/g, "'")
      .replace(/\s+/g, " ")
      .trim()
      .slice(0, 2000);

    return content;
  } catch {
    return null;
  }
}

// ========== イベントハンドラー ==========

// --- メンション応答 ---
// ボットがメンションされたとき（未購読スレッド）にClaude AIで応答する
bot.onNewMention(async (thread) => {
  try {
    await thread.refresh();
    const messageArray = thread.recentMessages.slice(-5);

    const conversationMessages = toConversationMessages(messageArray);
    if (conversationMessages.length === 0) {
      await thread.post("メッセージを取得できませんでした。");
      return;
    }

    await thread.post(streamAIResponse(conversationMessages));
  } catch (error) {
    console.error("Error in onNewMention:", error);
    await thread.post("申し訳ありません。エラーが発生しました。");
  }
});

// --- 購読済みスレッドのメッセージ応答 ---
// フォーラムチャンネルなど、購読済みスレッドのメッセージに応答する
bot.onSubscribedMessage(async (thread, message) => {
  if (message.author.isBot || message.author.isMe) return;

  try {
    const isForum = isTargetForum(thread.id);
    const limit = isForum ? 100 : 5;

    const messageArray = isForum
      ? await collectMessages(thread.messages, limit)
      : (await thread.refresh(), thread.recentMessages.slice(-limit));

    // /clear によるリセット時刻を考慮
    const resetTime = resetTimes.get(JSON.stringify(thread.id));
    const conversationMessages = toConversationMessages(messageArray);

    if (conversationMessages.length === 0) return;

    await thread.post(streamAIResponse(conversationMessages));
  } catch (error) {
    console.error("Error in onSubscribedMessage:", error);
    await thread.post("申し訳ありません。エラーが発生しました。");
  }
});

// --- フォーラム自動応答 ---
// 対象フォーラムチャンネルの新規メッセージに自動で応答する
bot.onNewMessage(/[\s\S]*/, async (thread, message) => {
  if (!isTargetForum(thread.id)) return;
  if (message.author.isBot || message.author.isMe) return;
  if (message.isMention) return; // メンションは onNewMention で処理

  try {
    await thread.subscribe();

    const messageArray = await collectMessages(thread.messages, 100);

    const conversationMessages = toConversationMessages(messageArray);
    if (conversationMessages.length === 0) return;

    await thread.post(streamAIResponse(conversationMessages));
  } catch (error) {
    console.error("Error in forum auto-response:", error);
    await thread.post("申し訳ありません。エラーが発生しました。");
  }
});

// --- イースターエッグ ---
bot.onNewMessage(/ぬるぽ/, async (thread) => {
  await thread.post("ガッ");
});

// --- 📝 リアクション要約 ---
bot.onReaction(["📝"], async (event) => {
  try {
    // リアクションされたメッセージを取得する
    await event.thread.refresh();
    const messageArray = event.thread.recentMessages.slice(-10);

    if (messageArray.length === 0) return;

    // リアクションのコンテキストとなるメッセージを取得
    const targetMessage = messageArray[0];
    const text = targetMessage.text || "";

    // URL を抽出（最大3件）
    const urlRegex = /https?:\/\/[^\s<>)]+/g;
    const urls = (text.match(urlRegex) || []).slice(0, 3);

    if (urls.length === 0 && !text.trim()) return;

    // URL コンテンツを並列取得
    const urlContents: { url: string; content: string }[] = [];
    const fetchPromises = urls.map(async (url: string) => {
      const content = await fetchUrlContent(url);
      if (content) {
        urlContents.push({ url, content });
      }
    });
    await Promise.all(fetchPromises);

    const summary = await generateUrlSummary(text, urlContents);

    await event.thread.post(
      Card({
        title: "要約",
        children: [CardText(summary)],
      }),
    );
  } catch (error) {
    console.error("Error in reaction handler:", error);
    await event.thread.post("要約の生成中にエラーが発生しました。");
  }
});

// ========== スラッシュコマンド ==========

// /help - コマンド一覧を表示
bot.onSlashCommand("help", async (event) => {
  await event.channel.post(
    Card({
      title: "IdealX ヘルプ",
      children: [
        Section([
          CardText("💬 **メンション機能**"),
          CardText("IdealXにメンションすると、Claude AIが直近の会話を読み取り回答します。"),
        ]),
        Divider(),
        Section([
          CardText("📋 **スラッシュコマンド**"),
          CardText(
            [
              "`/help` - このヘルプを表示",
              "`/age [ユーザー]` - Discordアカウント作成日と経過日数を表示",
              "`/summarize [件数]` - 直近メッセージをAI要約（デフォルト10件、最大50件）",
              "`/translate [言語] [テキスト]` - テキストを指定言語に翻訳",
              "`/clear` - 会話コンテキストをリセット",
            ].join("\n"),
          ),
        ]),
        Divider(),
        Section([
          CardText("⚡ **リアクション機能**"),
          CardText("📝リアクションでメッセージを要約してチャンネルに投稿"),
        ]),
        CardText("Powered by Claude claude-sonnet-4-6", { style: "muted" }),
      ],
    }),
  );
});

// /clear - 会話コンテキストをリセット
bot.onSlashCommand("clear", async (event) => {
  resetTimes.set(JSON.stringify(event.channel.id), new Date());
  await event.channel.post(
    Card({
      title: "コンテキストリセット",
      children: [
        CardText("会話コンテキストをリセットしました。これ以降のメッセージのみがAIへの入力として使用されます。"),
      ],
    }),
  );
});

// /summarize [count] - 直近メッセージを要約
bot.onSlashCommand("summarize", async (event) => {
  try {
    // コマンドテキストから件数を取得（デフォルト10、最大50）
    const rawCount = Number(event.text) || 10;
    const count = Math.min(Math.max(rawCount, 1), 50);

    // Channel の messages async iterable から指定件数を取得
    const allMessages = await collectMessages(event.channel.messages, count);
    const messageArray = allMessages.filter(
      (m) => !m.author?.isBot && !m.author?.isMe,
    );

    const conversationMessages = toConversationMessages(messageArray);

    if (conversationMessages.length === 0) {
      await event.channel.post("要約するメッセージが見つかりませんでした。");
      return;
    }

    const summary = await generateSummary(conversationMessages);
    await event.channel.post(
      Card({
        title: "会話の要約",
        children: [
          CardText(summary),
          Divider(),
          CardText(`${conversationMessages.length}件のメッセージを要約`, { style: "muted" }),
        ],
      }),
    );
  } catch (error) {
    console.error("Error in summarize command:", error);
    await event.channel.post("要約の生成中にエラーが発生しました。");
  }
});

// /translate [language] [text] - テキストを翻訳
const LANGUAGES: Record<string, string> = {
  japanese: "日本語",
  english: "英語",
  chinese_simplified: "中国語（簡体字）",
  chinese_traditional: "中国語（繁体字）",
  korean: "韓国語",
  french: "フランス語",
  german: "ドイツ語",
  spanish: "スペイン語",
  portuguese: "ポルトガル語",
  italian: "イタリア語",
  russian: "ロシア語",
  arabic: "アラビア語",
};

bot.onSlashCommand("translate", async (event) => {
  try {
    // event.text から "language text" 形式でパース
    const parts = event.text.trim().split(/\s+/);
    const languageKey = (parts[0] || "").toLowerCase();
    const text = parts.slice(1).join(" ");

    if (!text) {
      await event.channel.post("翻訳するテキストを入力してください。");
      return;
    }

    const language = LANGUAGES[languageKey] || languageKey;
    const translation = await generateTranslation(text, language);

    const displayOriginal = text.length > 500 ? text.slice(0, 500) + "…" : text;
    await event.channel.post(
      Card({
        title: `${language}への翻訳`,
        children: [
          Fields([
            Field({ label: "原文", value: displayOriginal }),
            Field({ label: "翻訳", value: translation }),
          ]),
        ],
      }),
    );
  } catch (error) {
    console.error("Error in translate command:", error);
    await event.channel.post("翻訳中にエラーが発生しました。");
  }
});

// /age [user] - アカウント作成日と経過日数
bot.onSlashCommand("age", async (event) => {
  try {
    const userId = event.text.trim();

    if (!userId) {
      await event.channel.post("ユーザーを指定してください。");
      return;
    }

    // Discord Snowflake からアカウント作成日を算出
    const DISCORD_EPOCH = 1420070400000n;
    const snowflake = BigInt(userId);
    const timestamp = Number((snowflake >> 22n) + DISCORD_EPOCH);
    const createdAt = new Date(timestamp);

    const now = new Date();
    const diffMs = now.getTime() - createdAt.getTime();
    const totalDays = Math.floor(diffMs / (1000 * 60 * 60 * 24));
    const years = Math.floor(totalDays / 365);
    const remainingDays = totalDays % 365;

    await event.channel.post(
      Card({
        title: "アカウント情報",
        children: [
          CardText(`<@${userId}>`),
          Fields([
            Field({ label: "作成日", value: `<t:${Math.floor(createdAt.getTime() / 1000)}:R>` }),
            Field({ label: "経過日数", value: `${totalDays}日（${years}年${remainingDays}日）` }),
          ]),
        ],
      }),
    );
  } catch (error) {
    console.error("Error in age command:", error);
    await event.channel.post("アカウント情報の取得中にエラーが発生しました。");
  }
});
