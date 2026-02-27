import { config } from "dotenv";
config();

import { Hono } from "hono";
import { serve } from "@hono/node-server";
import { bot, initializeBot } from "./lib/bot.js";

const app = new Hono();
const port = Number(process.env.PORT) || 3000;

// Discord Webhook エンドポイント
app.post("/api/webhooks/discord", async (c) => {
  const response = await bot.webhooks.discord(c.req.raw, {
    waitUntil: (promise: Promise<unknown>) => {
      promise.catch(console.error);
    },
  });
  return response;
});

// ヘルスチェック
app.get("/", (c) => c.text("IdealX Bot is running"));
app.get("/api/webhooks/discord", (c) =>
  c.text("Discord webhook endpoint active"),
);

/**
 * Discord Gateway リスナーを起動する。
 * Gateway WebSocket で受信したメッセージ・リアクションイベントを
 * Webhook エンドポイントに転送する。
 */
async function startGatewayListener() {
  const discordAdapter = bot.getAdapter("discord");

  if (!discordAdapter || !("startGatewayListener" in discordAdapter)) {
    console.warn(
      "Discord adapter does not support gateway listener. Skipping.",
    );
    return;
  }

  const webhookUrl = `http://localhost:${port}/api/webhooks/discord`;

  // 切断時に自動再接続するループ
  while (true) {
    try {
      console.log("Starting Discord gateway listener...");
      let resolveGateway!: () => void;
      const gatewayDone = new Promise<void>((resolve) => {
        resolveGateway = resolve;
      });

      const adapter = discordAdapter as {
        startGatewayListener: (
          options: { waitUntil: (p: Promise<unknown>) => void },
          durationMs: number,
          abortSignal: undefined,
          webhookUrl: string,
        ) => Promise<unknown>;
      };
      await adapter.startGatewayListener(
        {
          waitUntil: (promise: Promise<unknown>) => {
            promise.then(resolveGateway, resolveGateway);
          },
        },
        24 * 60 * 60 * 1000, // 24時間
        undefined,
        webhookUrl,
      );

      // Gateway が終了するまで待機
      await gatewayDone;

      console.log("Gateway listener finished, restarting...");
    } catch (error) {
      console.error("Gateway listener error:", error);
      // 再接続まで5秒待機
      await new Promise((resolve) => setTimeout(resolve, 5000));
    }
  }
}

async function main() {
  await initializeBot();

  serve({ fetch: app.fetch, port }, () => {
    console.log(`IdealX server running on port ${port}`);
  });

  // Gateway リスナーをバックグラウンドで起動
  startGatewayListener();
}

// グレースフルシャットダウン
process.on("SIGINT", () => {
  console.log("Shutting down...");
  process.exit(0);
});

process.on("SIGTERM", () => {
  console.log("Shutting down...");
  process.exit(0);
});

main().catch(console.error);
