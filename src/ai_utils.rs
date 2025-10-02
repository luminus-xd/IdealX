use serde::Serialize;

#[derive(Serialize, Clone, Debug)]
pub struct RequestMessage<'a> {
    pub role: &'a str,
    pub content: String,
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
