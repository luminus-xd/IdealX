use reqwest;
use reqwest::Error;
use serde::{Deserialize, Serialize};

#[derive(Deserialize, Debug)]
struct ClaudeResponse {
    content: Vec<ContentBlock>,
}

#[derive(Deserialize, Debug)]
struct ContentBlock {
    #[serde(rename = "type")]
    content_type: String,
    text: String,
}

#[derive(Serialize)]
struct ClaudeRequest<'a> {
    model: &'a str,
    messages: Vec<RequestMessage<'a>>,
    max_tokens: u32,
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
) -> Result<String, Error> {
    const URL: &str = "https://api.anthropic.com/v1/messages";
    const CLAUDE_MODEL: &str = "claude-3-7-sonnet-20250219"; // Claudeのモデル名
    const MAX_TOKENS: u32 = 4096; // 最大トークン数

    let request_body = ClaudeRequest {
        model: CLAUDE_MODEL,
        messages,
        max_tokens: MAX_TOKENS,
    };

    let response = client
        .post(URL)
        .header("Content-Type", "application/json")
        .header("x-api-key", claude_token)
        .header("anthropic-version", "2023-06-01")
        .json(&request_body) // リクエストボディをJSONに変換
        .send()
        .await?
        .json::<ClaudeResponse>() // レスポンスをClaudeResponse構造体にデシリアライズ
        .await?;

    // テキストコンテンツを結合
    let content = response.content.iter()
        .filter(|block| block.content_type == "text")
        .map(|block| block.text.clone())
        .collect::<Vec<String>>()
        .join("");

    log::info!("Response content length: {}", content.len());

    return Ok(content);
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
            if let Some(last_period) = substring.rfind(|c| c == '。' || c == '.' || c == '!' || c == '?' || c == '\n') {
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