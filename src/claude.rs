use serde::Deserialize;
use tracing::{error, info};

use crate::artifact::store::{ArtifactLink, ArtifactStore, MAX_TITLE_CHARS};

#[derive(Debug)]
pub enum ClaudeError {
    HttpError(reqwest::Error),
    ApiError(String),
    ParseError(String),
}

impl From<reqwest::Error> for ClaudeError {
    fn from(error: reqwest::Error) -> Self {
        ClaudeError::HttpError(error)
    }
}

impl std::fmt::Display for ClaudeError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ClaudeError::HttpError(e) => write!(f, "HTTP error: {}", e),
            ClaudeError::ApiError(e) => write!(f, "API error: {}", e),
            ClaudeError::ParseError(e) => write!(f, "Parse error: {}", e),
        }
    }
}

impl std::error::Error for ClaudeError {}

#[derive(Clone, Debug)]
pub enum ContentBlock {
    Text(String),
    ImageUrl(String),
}

impl ContentBlock {
    fn to_json(&self) -> serde_json::Value {
        match self {
            ContentBlock::Text(text) => serde_json::json!({
                "type": "text",
                "text": text
            }),
            ContentBlock::ImageUrl(url) => serde_json::json!({
                "type": "image",
                "source": {
                    "type": "url",
                    "url": url
                }
            }),
        }
    }
}

#[derive(Clone, Debug)]
pub enum MessageContent {
    Text(String),
    Blocks(Vec<ContentBlock>),
}

impl MessageContent {
    pub fn to_json(&self) -> serde_json::Value {
        match self {
            MessageContent::Text(s) => serde_json::Value::String(s.clone()),
            MessageContent::Blocks(blocks) => {
                serde_json::Value::Array(blocks.iter().map(|b| b.to_json()).collect())
            }
        }
    }

    pub fn append_text(&mut self, extra: &str) {
        match self {
            MessageContent::Text(s) => {
                s.push_str("\n\n");
                s.push_str(extra);
            }
            MessageContent::Blocks(blocks) => {
                // 最後のテキストブロックに追記、なければ新規追加
                if let Some(ContentBlock::Text(last)) = blocks.last_mut() {
                    last.push_str("\n\n");
                    last.push_str(extra);
                } else {
                    blocks.push(ContentBlock::Text(extra.to_string()));
                }
            }
        }
    }
}

#[derive(Clone, Debug)]
pub struct RequestMessage<'a> {
    pub role: &'a str,
    pub content: MessageContent,
}

/// Claudeの応答と、応答中に作られたアーティファクト
#[derive(Debug, Default)]
pub struct ClaudeReply {
    pub text: String,
    pub artifacts: Vec<ArtifactLink>,
}

const ARTIFACT_TOOL_NAME: &str = "create_artifact";

#[derive(Deserialize)]
struct CreateArtifactInput {
    title: String,
    html: String,
}

fn artifact_tool_definition(store: &ArtifactStore) -> serde_json::Value {
    let ttl_hours = store.ttl().as_secs() / 3600;
    serde_json::json!({
        "name": ARTIFACT_TOOL_NAME,
        "description": format!(
            "単体で動くHTMLページを作り、誰でもブラウザで開ける公開URLを返す。\
             Discordのテキストでは表現しにくいもの（表、グラフ、図解、計算ツール、ミニゲーム、\
             インタラクティブなデモ、整形された長い資料など）を見せたいとき、またはユーザーがページや\
             アーティファクトを作ってほしいと頼んだときに使う。短い回答やコード例だけで足りる場合は使わない。\
             制約: CSSとJavaScriptはインラインで書く。外部スクリプトとスタイルは cdnjs.cloudflare.com / \
             cdn.jsdelivr.net / unpkg.com、フォントは Google Fonts だけ読み込める。\
             fetch / XHR / WebSocket などの通信、localStorage / sessionStorage / Cookie、iframe、\
             フォーム送信は使えない（ページはサンドボックス内で動く）。画像は https か data: URI。\
             スマートフォン幅でも崩れないレイアウトにする。ページは約{ttl_hours}時間で消え、\
             作った後の編集はできないので、直すときは新しく作り直す。\
             返ってきたURLは返信本文に必ず載せる。"
        ),
        "strict": true,
        "input_schema": {
            "type": "object",
            "properties": {
                "title": {
                    "type": "string",
                    "description": format!("ページの短いタイトル（{MAX_TITLE_CHARS}文字以内）")
                },
                "html": {
                    "type": "string",
                    "description": "<!doctype html> から始まる完結したHTML文書"
                }
            },
            "required": ["title", "html"],
            "additionalProperties": false
        }
    })
}

/// create_artifact の呼び出しを実行し、tool_result ブロックを返す
fn run_artifact_tool(
    block: &serde_json::Value,
    store: Option<&ArtifactStore>,
    created: &mut Vec<ArtifactLink>,
) -> serde_json::Value {
    let tool_use_id = block["id"].clone();
    let error = |message: String| {
        serde_json::json!({
            "type": "tool_result",
            "tool_use_id": tool_use_id,
            "is_error": true,
            "content": message
        })
    };

    let Some(store) = store.filter(|_| block["name"] == ARTIFACT_TOOL_NAME) else {
        return error(format!("不明なツールです: {}", block["name"]));
    };
    let input: CreateArtifactInput = match serde_json::from_value(block["input"].clone()) {
        Ok(input) => input,
        Err(e) => return error(format!("入力が不正です: {e}")),
    };

    match store.create(&input.title, &input.html) {
        Ok(link) => {
            info!("Created artifact {} ({} bytes)", link.id, input.html.len());
            let content = format!(
                "ページを公開しました。\nURL: {}\n有効期限: 約{}時間",
                link.url,
                store.ttl().as_secs() / 3600
            );
            created.push(link);
            serde_json::json!({
                "type": "tool_result",
                "tool_use_id": tool_use_id,
                "content": content
            })
        }
        Err(e) => error(format!("ページを作れませんでした: {e}")),
    }
}

/// Claudeにリクエストを送信、レスポンスを取得（ウェブ検索ツール付き）
pub async fn get_claude_response(
    messages: Vec<RequestMessage<'_>>, // リクエストに含めるメッセージのベクター
    claude_token: &str,                // Claude APIのアクセストークン
    client: &reqwest::Client,          // reqwestのクライアントインスタンス
    system_prompt: Option<&str>,       // システムプロンプト
) -> Result<String, ClaudeError> {
    get_claude_reply(messages, claude_token, client, system_prompt, None)
        .await
        .map(|reply| reply.text)
}

/// Claudeにリクエストを送信する。`artifacts` を渡すとアーティファクト作成ツールも使える
pub async fn get_claude_reply(
    messages: Vec<RequestMessage<'_>>,
    claude_token: &str,
    client: &reqwest::Client,
    system_prompt: Option<&str>,
    artifacts: Option<&ArtifactStore>,
) -> Result<ClaudeReply, ClaudeError> {
    const URL: &str = "https://api.anthropic.com/v1/messages";
    const CLAUDE_MODEL: &str = "claude-opus-5-5";
    const MAX_ITERATIONS: u8 = 6;
    // HTMLはツール入力として出力トークンを消費するので、アーティファクト有効時は上限を広げる
    let max_tokens: u32 = if artifacts.is_some() { 16_000 } else { 4_096 };

    // メッセージをJSON形式に変換（アジェンティックループ用）
    let mut messages_json: Vec<serde_json::Value> = messages
        .iter()
        .map(|m| {
            serde_json::json!({
                "role": m.role,
                "content": m.content.to_json()
            })
        })
        .collect();

    // ウェブ検索ツールの定義
    let mut tools = vec![serde_json::json!({
        "type": "web_search_20250305",
        "name": "web_search",
        "max_uses": 5
    })];
    if let Some(store) = artifacts {
        tools.push(artifact_tool_definition(store));
    }

    info!(
        "Sending request to Claude API with {} messages and {} tools",
        messages_json.len(),
        tools.len()
    );

    let mut created_artifacts = Vec::new();

    for iteration in 0..MAX_ITERATIONS {
        let mut request_body = serde_json::json!({
            "model": CLAUDE_MODEL,
            "max_tokens": max_tokens,
            "messages": messages_json,
            "tools": tools
        });

        if let Some(system) = system_prompt {
            request_body["system"] = serde_json::json!(system);
        }

        let http_response = client
            .post(URL)
            .header("Content-Type", "application/json")
            .header("x-api-key", claude_token)
            .header("anthropic-version", "2023-06-01")
            .json(&request_body)
            .send()
            .await?;

        let status = http_response.status();
        info!("Claude API responded with status: {}", status);

        if !status.is_success() {
            let error_text = http_response.text().await?;
            error!("Claude API error response: {}", error_text);
            return Err(ClaudeError::ApiError(format!(
                "Claude API error: {} - {}",
                status, error_text
            )));
        }

        let response_json: serde_json::Value = http_response
            .json()
            .await
            .map_err(|e| ClaudeError::ParseError(format!("JSON parse error: {}", e)))?;

        info!(
            "Claude API response (iteration {}): stop_reason={}",
            iteration, response_json["stop_reason"]
        );

        // APIエラーの確認
        if let Some(error) = response_json["error"].as_object() {
            let msg = error
                .get("message")
                .and_then(|v| v.as_str())
                .unwrap_or("Unknown error");
            error!("Claude API returned error: {}", msg);
            return Err(ClaudeError::ApiError(format!("Claude API error: {}", msg)));
        }

        let stop_reason = response_json["stop_reason"].as_str().unwrap_or("end_turn");
        let blocks = response_json["content"]
            .as_array()
            .cloned()
            .unwrap_or_default();

        // テキストコンテンツを抽出
        let text_content: String = blocks
            .iter()
            .filter(|b| b["type"] == "text")
            .filter_map(|b| b["text"].as_str())
            .collect::<Vec<_>>()
            .join("");

        match stop_reason {
            "end_turn" => {
                info!("Response content length: {}", text_content.len());
                return Ok(ClaudeReply {
                    text: text_content,
                    artifacts: created_artifacts,
                });
            }
            "pause_turn" => {
                // pause_turn: レスポンスをアシスタントターンとして追加し、ループを継続
                info!("Got pause_turn, continuing conversation...");
                messages_json.push(serde_json::json!({
                    "role": "assistant",
                    "content": blocks
                }));
                continue;
            }
            "tool_use" => {
                // クライアント側ツール（create_artifact）を実行し、結果をまとめて1つのuserターンで返す
                let tool_results: Vec<serde_json::Value> = blocks
                    .iter()
                    .filter(|b| b["type"] == "tool_use")
                    .map(|b| run_artifact_tool(b, artifacts, &mut created_artifacts))
                    .collect();
                if tool_results.is_empty() {
                    error!("tool_use stop_reason without client tool_use blocks");
                    return Err(ClaudeError::ApiError(
                        "Unexpected tool_use stop_reason without tool calls".to_string(),
                    ));
                }
                messages_json.push(serde_json::json!({
                    "role": "assistant",
                    "content": blocks
                }));
                messages_json.push(serde_json::json!({
                    "role": "user",
                    "content": tool_results
                }));
                continue;
            }
            other => {
                info!("Unexpected stop_reason: {}, returning text content", other);
                return Ok(ClaudeReply {
                    text: text_content,
                    artifacts: created_artifacts,
                });
            }
        }
    }

    Err(ClaudeError::ApiError(
        "ツール呼び出しループの最大試行回数を超えました".to_string(),
    ))
}

/// Discordのメッセージ制限（2000文字）に合わせてメッセージを分割する
pub fn split_message(message: &str, max_length: usize) -> Vec<String> {
    if message.len() <= max_length {
        return vec![message.to_string()];
    }

    let mut result = Vec::new();
    let mut current_pos = 0;

    while current_pos < message.len() {
        let mut end_pos = std::cmp::min(current_pos + max_length, message.len());

        // 文字の途中で切らないように調整
        if end_pos < message.len() {
            // 最後の空白または改行を探す
            while end_pos > current_pos && !message.is_char_boundary(end_pos) {
                end_pos -= 1;
            }

            // 文の途中で切らないように、最後の文章の区切りを探す
            let substring = &message[current_pos..end_pos];
            if let Some(last_period) = substring.rfind(['。', '.', '!', '?', '\n']) {
                end_pos = current_pos + last_period + 1;

                // 文字境界でない場合は調整
                while end_pos > current_pos && !message.is_char_boundary(end_pos) {
                    end_pos -= 1;
                }
            }
        }

        result.push(message[current_pos..end_pos].to_string());
        current_pos = end_pos;
    }

    result
}
