use serde::Serialize;
use tracing::{error, info};

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

#[derive(Serialize, Clone, Debug)]
pub struct RequestMessage<'a> {
    pub role: &'a str,
    pub content: String,
}

/// Claudeにリクエストを送信、レスポンスを取得（ウェブ検索ツール付き）
pub async fn get_claude_response(
    messages: Vec<RequestMessage<'_>>, // リクエストに含めるメッセージのベクター
    claude_token: &str,                // Claude APIのアクセストークン
    client: &reqwest::Client,          // reqwestのクライアントインスタンス
    system_prompt: Option<&str>,       // システムプロンプト
) -> Result<String, ClaudeError> {
    const URL: &str = "https://api.anthropic.com/v1/messages";
    const CLAUDE_MODEL: &str = "claude-sonnet-4-6";
    const MAX_TOKENS: u32 = 4096;
    const MAX_ITERATIONS: u8 = 6;

    // メッセージをJSON形式に変換（アジェンティックループ用）
    let mut messages_json: Vec<serde_json::Value> = messages
        .iter()
        .map(|m| {
            serde_json::json!({
                "role": m.role,
                "content": m.content
            })
        })
        .collect();

    // ウェブ検索ツールの定義
    let tools = serde_json::json!([{
        "type": "web_search_20250305",
        "name": "web_search",
        "max_uses": 5
    }]);

    info!(
        "Sending request to Claude API with {} messages and web search tool",
        messages_json.len()
    );

    for iteration in 0..MAX_ITERATIONS {
        let mut request_body = serde_json::json!({
            "model": CLAUDE_MODEL,
            "max_tokens": MAX_TOKENS,
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

        // テキストコンテンツを抽出
        let text_content: String = response_json["content"]
            .as_array()
            .map(|blocks| {
                blocks
                    .iter()
                    .filter(|b| b["type"] == "text")
                    .filter_map(|b| b["text"].as_str())
                    .collect::<Vec<_>>()
                    .join("")
            })
            .unwrap_or_default();

        match stop_reason {
            "end_turn" => {
                info!("Response content length: {}", text_content.len());
                return Ok(text_content);
            }
            "pause_turn" => {
                // pause_turn: レスポンスをアシスタントターンとして追加し、ループを継続
                info!("Got pause_turn, continuing conversation...");
                let assistant_content = response_json["content"].clone();
                messages_json.push(serde_json::json!({
                    "role": "assistant",
                    "content": assistant_content
                }));
                continue;
            }
            "tool_use" => {
                // tool_use: web_search_20250305はサーバーサイドツールのため通常発生しないが、
                // 万一返ってきた場合はエラーとして扱う
                error!("Unexpected tool_use stop_reason received (server-side tool should not require client handling)");
                return Err(ClaudeError::ApiError(
                    "Unexpected tool_use stop_reason from server-side tool".to_string(),
                ));
            }
            other => {
                info!("Unexpected stop_reason: {}, returning text content", other);
                return Ok(text_content);
            }
        }
    }

    Err(ClaudeError::ApiError(
        "ウェブ検索ループの最大試行回数を超えました".to_string(),
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
