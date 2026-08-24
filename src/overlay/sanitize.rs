use regex::Regex;
use std::sync::LazyLock;
use unicode_normalization::UnicodeNormalization;
use unicode_segmentation::UnicodeSegmentation;

const MAX_BODY_GRAPHEMES: usize = 300;
const MAX_BODY_LINES: usize = 3;
const MAX_AUTHOR_GRAPHEMES: usize = 80;

static URL_RE: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r#"(?i)\b(?:https?://|www\.)[^\s<>{}\[\]\"']+"#)
        .expect("overlay URL regex must compile")
});

/// Discord本文をOBSで安全に表示できるプレーンテキストへ変換する。
pub fn sanitize_body(input: &str) -> String {
    let normalized = normalize_and_strip(input);
    let without_urls = URL_RE.replace_all(&normalized, "[link]");

    let lines = without_urls
        .lines()
        .map(collapse_inline_whitespace)
        .filter(|line| !line.is_empty())
        .take(MAX_BODY_LINES)
        .collect::<Vec<_>>();

    truncate_graphemes(&lines.join("\n"), MAX_BODY_GRAPHEMES)
}

/// 表示名を単一行の安全なプレーンテキストへ変換する。
pub fn sanitize_author_name(input: &str) -> String {
    let normalized = normalize_and_strip(input);
    let single_line = collapse_inline_whitespace(&normalized.replace(['\r', '\n'], " "));
    truncate_graphemes(&single_line, MAX_AUTHOR_GRAPHEMES)
}

/// Discord CDN上の静的アバターだけを許可する。
pub fn sanitize_avatar_url(input: Option<&str>) -> Option<String> {
    let raw = input?;
    let path_and_query = raw
        .strip_prefix("https://cdn.discordapp.com")
        .or_else(|| raw.strip_prefix("https://cdn.discordapp.net"))?;
    let path = path_and_query.split(['?', '#']).next()?;
    if !(path.starts_with("/avatars/") || path.starts_with("/embed/avatars/")) {
        return None;
    }

    let is_static_image = [".png", ".jpg", ".jpeg", ".webp"]
        .iter()
        .any(|extension| path.to_ascii_lowercase().ends_with(extension));

    is_static_image.then(|| raw.to_string())
}

fn normalize_and_strip(input: &str) -> String {
    input
        .replace("\r\n", "\n")
        .replace('\r', "\n")
        .nfc()
        .filter_map(|character| {
            if is_bidi_control(character) {
                None
            } else if character == '\n' {
                Some(character)
            } else if character == '\t' {
                Some(' ')
            } else if character.is_control() {
                None
            } else {
                Some(character)
            }
        })
        .collect()
}

fn is_bidi_control(character: char) -> bool {
    matches!(
        character,
        '\u{061C}'
            | '\u{200E}'
            | '\u{200F}'
            | '\u{202A}'..='\u{202E}'
            | '\u{2066}'..='\u{2069}'
    )
}

fn collapse_inline_whitespace(input: &str) -> String {
    input.split_whitespace().collect::<Vec<_>>().join(" ")
}

fn truncate_graphemes(input: &str, max: usize) -> String {
    let graphemes = UnicodeSegmentation::graphemes(input, true).collect::<Vec<_>>();
    if graphemes.len() <= max {
        return input.to_string();
    }

    if max == 0 {
        return String::new();
    }

    let mut truncated = graphemes[..max - 1].concat();
    truncated.push('…');
    truncated
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn body_is_normalized_and_dangerous_characters_are_removed() {
        let input = "Cafe\u{301}\u{202E}<script>alert(1)</script>\u{0007}";

        let result = sanitize_body(input);

        assert_eq!(result, "Café<script>alert(1)</script>");
        assert!(!result.contains('\u{202E}'));
        assert!(!result.contains('\u{0007}'));
    }

    #[test]
    fn urls_are_replaced_without_making_surrounding_text_disappear() {
        let result =
            sanitize_body("docs: https://example.com/path?q=1 and www.example.org/page. 次です");

        assert_eq!(result, "docs: [link] and [link] 次です");
    }

    #[test]
    fn body_is_limited_to_three_lines() {
        let result = sanitize_body("one\ntwo\nthree\nfour");

        assert_eq!(result, "one\ntwo\nthree");
    }

    #[test]
    fn body_is_limited_by_grapheme_cluster_not_scalar_value() {
        let family = "👨‍👩‍👧‍👦";
        let result = sanitize_body(&family.repeat(301));

        assert_eq!(
            UnicodeSegmentation::graphemes(result.as_str(), true).count(),
            300
        );
        assert!(result.ends_with('…'));
    }

    #[test]
    fn author_name_is_single_line_and_limited() {
        let result = sanitize_author_name(&format!(" Alice\nBob {} ", "x".repeat(100)));

        assert!(!result.contains('\n'));
        assert_eq!(
            UnicodeSegmentation::graphemes(result.as_str(), true).count(),
            80
        );
    }

    #[test]
    fn only_static_discord_cdn_avatars_are_allowed() {
        assert!(sanitize_avatar_url(Some(
            "https://cdn.discordapp.com/avatars/1/hash.webp?size=128"
        ))
        .is_some());
        assert!(
            sanitize_avatar_url(Some("https://cdn.discordapp.com/avatars/1/hash.gif")).is_none()
        );
        assert!(sanitize_avatar_url(Some("https://example.com/avatar.webp")).is_none());
    }
}
