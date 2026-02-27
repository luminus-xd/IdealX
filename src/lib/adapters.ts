import { createDiscordAdapter } from "@chat-adapter/discord";

/**
 * Discord アダプターを構築する。
 * 環境変数から自動的に認証情報を読み取る:
 * - DISCORD_BOT_TOKEN
 * - DISCORD_PUBLIC_KEY
 * - DISCORD_APPLICATION_ID
 * - DISCORD_MENTION_ROLE_IDS (任意)
 */
export function buildAdapters() {
  return {
    discord: createDiscordAdapter(),
  };
}

export type Adapters = ReturnType<typeof buildAdapters>;
