use reqwest;
use reqwest::Error;
use serde::{Deserialize, Serialize};

#[derive(Deserialize, Debug)]
struct ChatResponse {
    choices: Vec<Choice>,
}

#[derive(Deserialize, Debug)]
struct Choice {
    message: ResponseMessage,
}

#[derive(Deserialize, Debug)]
struct ResponseMessage {
    role: String,
    content: String,
}

#[derive(Serialize)]
struct ChatRequest<'a> {
    model: &'a str,
    messages: Vec<RequestMessage<'a>>,
}

#[derive(Serialize, Clone, Debug)]
pub struct RequestMessage<'a> {
    pub role: &'a str,
    pub content: String,
}

/// GPTにリクエストを送信、レスポンスを取得
pub async fn get_gpt_response(
    messages: Vec<RequestMessage<'_>>, // リクエストに含めるメッセージのベクター
    gpt_token: &str,                   // ChatGPT APIのアクセストークン
    client: &reqwest::Client,          // reqwestのクライアントインスタンス
) -> Result<String, Error> {
    const URL: &str = "https://api.openai.com/v1/chat/completions";
    const GPT_MODEL: &str = "gpt-4-turbo-2024-04-09"; // GPTのモデル名

    let request_body = ChatRequest {
        model: GPT_MODEL,
        messages,
    };

    let response = client
        .post(URL)
        .header("Content-Type", "application/json")
        .header("Authorization", format!("Bearer {}", gpt_token))
        .json(&request_body) // リクエストボディをJSONに変換
        .send()
        .await?
        .json::<ChatResponse>() // レスポンスをChatResponse構造体にデシリアライズ
        .await?;

    log::info!("Response content: {}", response.choices[0].message.content);

    return Ok(response.choices[0].message.content.clone());
}
