import { generateText, stepCountIs } from "ai";
import { anthropic } from "@ai-sdk/anthropic";
import { readFileSync } from "node:fs";
import { resolve } from "node:path";

// システムプロンプトをファイルから読み込み
const systemPrompt = readFileSync(
  resolve(process.cwd(), "system_prompt.md"),
  "utf-8",
);

const model = anthropic("claude-sonnet-4-6");

const webSearchTool = anthropic.tools.webSearch_20250305({
  maxUses: 5,
});

export interface ConversationMessage {
  role: "user" | "assistant";
  content: string;
}

/**
 * Claude AI でレスポンスを生成する（ウェブ検索対応）
 */
export async function generateAIResponse(
  messages: ConversationMessage[],
  options?: {
    forumTitle?: string;
    forumDescription?: string;
  },
): Promise<string> {
  let system = systemPrompt;

  if (options?.forumTitle || options?.forumDescription) {
    system += "\n\n--- フォーラム情報 ---";
    if (options.forumTitle) system += `\nタイトル: ${options.forumTitle}`;
    if (options.forumDescription)
      system += `\n説明: ${options.forumDescription}`;
  }

  const result = await generateText({
    model,
    system,
    messages,
    tools: { web_search: webSearchTool },
    stopWhen: stepCountIs(6),
    maxOutputTokens: 4096,
  });

  return result.text;
}

/**
 * 会話メッセージの要約を生成する
 */
export async function generateSummary(
  messages: ConversationMessage[],
): Promise<string> {
  const prompt = messages
    .map(
      (m) =>
        `${m.role === "user" ? "ユーザー" : "ボット"}: ${m.content}`,
    )
    .join("\n");

  const result = await generateText({
    model,
    system:
      "以下の会話を簡潔に要約してください。要約のみを出力してください。",
    messages: [{ role: "user", content: prompt }],
    maxOutputTokens: 4096,
  });

  return result.text;
}

/**
 * テキストを指定言語に翻訳する
 */
export async function generateTranslation(
  text: string,
  language: string,
): Promise<string> {
  const result = await generateText({
    model,
    system: `以下のテキストを${language}に翻訳してください。翻訳文のみを出力してください。`,
    messages: [{ role: "user", content: text }],
    maxOutputTokens: 4096,
  });

  return result.text;
}

/**
 * URL内容を含むメッセージの要約を生成する（📝リアクション用）
 */
export async function generateUrlSummary(
  messageText: string,
  urlContents: { url: string; content: string }[],
): Promise<string> {
  let prompt = `以下のメッセージとURLの内容を要約してください。\n\nメッセージ: ${messageText}`;

  for (const { url, content } of urlContents) {
    prompt += `\n\n--- ${url} ---\n${content}`;
  }

  const result = await generateText({
    model,
    system:
      "メッセージの内容とURLの情報を簡潔にまとめてください。要約のみを出力してください。",
    messages: [{ role: "user", content: prompt }],
    maxOutputTokens: 4096,
  });

  return result.text;
}
