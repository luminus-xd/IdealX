use serde::{Deserialize, Serialize};
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

#[derive(Deserialize, Debug)]
struct ClaudeResponse {
    content: Vec<ContentBlock>,
    #[serde(default)]
    error: Option<ApiError>,
}

#[derive(Deserialize, Debug)]
struct ApiError {
    #[serde(rename = "type")]
    error_type: String,
    message: String,
}

#[derive(Deserialize, Debug)]
struct ContentBlock {
    #[serde(rename = "type")]
    content_type: String,
    #[serde(default)]
    text: Option<String>,
}

#[derive(Serialize)]
struct ClaudeRequest<'a> {
    model: &'a str,
    messages: Vec<RequestMessage<'a>>,
    max_tokens: u32,
    #[serde(skip_serializing_if = "Option::is_none")]
    system: Option<&'a str>,
}

#[derive(Serialize, Clone, Debug)]
pub struct RequestMessage<'a> {
    pub role: &'a str,
    pub content: String,
}

/// Claudeにリクエストを送信、レスポンスを取得
pub async fn get_claude_response(
    messages: Vec<RequestMessage<'_>>, // リクエストに含めるメッセージのベクター
    claude_token: &str,                // Claude APIのアクセストークン
    client: &reqwest::Client,          // reqwestのクライアントインスタンス
    system_prompt: Option<&str>,       // システムプロンプト
) -> Result<String, ClaudeError> {
    const URL: &str = "https://api.anthropic.com/v1/messages";
    const CLAUDE_MODEL: &str = "claude-opus-4-5-20251101"; // Claudeのモデル名
    const MAX_TOKENS: u32 = 4096; // 最大トークン数

    let request_body = ClaudeRequest {
        model: CLAUDE_MODEL,
        messages,
        max_tokens: MAX_TOKENS,
        system: system_prompt,
    };

    info!(
        "Sending request to Claude API with {} messages",
        request_body.messages.len()
    );

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

    let response_text = http_response.text().await?;
    info!("Raw Claude API response: {}", response_text);

    let response: ClaudeResponse = serde_json::from_str(&response_text).map_err(|e| {
        error!(
            "Failed to parse Claude response: {} - Response: {}",
            e, response_text
        );
        ClaudeError::ParseError(format!("JSON parse error: {}", e))
    })?;

    // エラーレスポンスの確認
    if let Some(error) = response.error {
        error!(
            "Claude API returned error: {} - {}",
            error.error_type, error.message
        );
        return Err(ClaudeError::ApiError(format!(
            "Claude API error: {}",
            error.message
        )));
    }

    // テキストコンテンツを結合
    let content = response
        .content
        .iter()
        .filter(|block| block.content_type == "text")
        .filter_map(|block| block.text.as_ref())
        .cloned()
        .collect::<Vec<String>>()
        .join("");

    info!("Response content length: {}", content.len());

    Ok(content)
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
