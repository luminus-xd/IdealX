use serde::{de::Error as _, Deserialize, Deserializer, Serialize, Serializer};
use std::time::Duration;

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum OverlayTheme {
    Dark,
    Light,
    Compact,
}

/// Discord上で開始されたオーバーレイセッションの状態。
#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SessionStatus {
    Active,
    Paused,
}

/// オーバーレイセッションの開始に必要な識別情報。
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct StartSession {
    pub guild_id: u64,
    pub channel_id: u64,
    pub owner_user_id: u64,
    pub theme: OverlayTheme,
    pub show_avatar: bool,
    pub display_duration: Duration,
}

/// OBSへ渡す表示URLを組み立てるための一時的な認証情報。
///
/// `capability` は平文のまま永続化・ログ出力しないこと。
#[derive(Clone, Eq, PartialEq)]
pub struct SessionCredentials {
    pub public_id: String,
    pub capability: String,
}

impl std::fmt::Debug for SessionCredentials {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("SessionCredentials")
            .field("public_id", &self.public_id)
            .field("capability", &"[REDACTED]")
            .finish()
    }
}

/// Discordコマンドの `/overlay status` などで参照する概要。
#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub struct SessionSummary {
    pub public_id: String,
    pub guild_id: u64,
    pub channel_id: u64,
    pub owner_user_id: u64,
    pub status: SessionStatus,
    pub revision: u64,
    pub comment_count: usize,
    pub expires_at_unix_ms: i64,
    pub theme: OverlayTheme,
    pub show_avatar: bool,
}

/// GatewayのMessageCreateからHubへ渡す、表示候補のコメント。
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct NewComment {
    pub guild_id: u64,
    pub channel_id: u64,
    pub discord_message_id: u64,
    pub author_user_id: u64,
    pub author_name: String,
    pub avatar_url: Option<String>,
    pub body: String,
    /// Discordの投稿時刻。取得できない場合は `None` とし、Hub側の現在時刻を使う。
    pub created_at_unix_ms: Option<i64>,
}

/// OBSへ実際に配信する、安全化済みコメント。
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct OverlayComment {
    #[serde(
        serialize_with = "serialize_u64_as_string",
        deserialize_with = "deserialize_u64_from_string"
    )]
    pub discord_message_id: u64,
    pub author_name: String,
    pub avatar_url: Option<String>,
    pub body: String,
    #[serde(
        rename = "created_at",
        serialize_with = "serialize_unix_ms_as_rfc3339",
        deserialize_with = "deserialize_unix_ms_from_rfc3339"
    )]
    pub created_at_unix_ms: i64,
    #[serde(
        rename = "expires_at",
        serialize_with = "serialize_unix_ms_as_rfc3339",
        deserialize_with = "deserialize_unix_ms_from_rfc3339"
    )]
    pub expires_at_unix_ms: i64,
}

/// WebSocket接続時に返す、セッションの完全な現在状態。
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct Snapshot {
    pub revision: u64,
    pub status: SessionStatus,
    pub comments: Vec<OverlayComment>,
    pub theme: OverlayTheme,
    pub show_avatar: bool,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SessionEndReason {
    Ended,
    Expired,
    CapabilityRotated,
    SyncLost,
}

/// Hubから各OBS Browser Sourceへbroadcastする差分イベント。
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum OverlayEvent {
    Snapshot {
        #[serde(flatten)]
        snapshot: Snapshot,
    },
    CommentAdded {
        revision: u64,
        comment: OverlayComment,
    },
    CommentUpdated {
        revision: u64,
        comment: OverlayComment,
    },
    CommentRemoved {
        revision: u64,
        #[serde(
            serialize_with = "serialize_u64_as_string",
            deserialize_with = "deserialize_u64_from_string"
        )]
        discord_message_id: u64,
    },
    Cleared {
        revision: u64,
    },
    Paused {
        revision: u64,
    },
    Resumed {
        revision: u64,
    },
    SessionEnded {
        revision: u64,
        reason: SessionEndReason,
    },
}

impl OverlayEvent {
    pub fn revision(&self) -> u64 {
        match self {
            Self::Snapshot { snapshot } => snapshot.revision,
            Self::CommentAdded { revision, .. }
            | Self::CommentUpdated { revision, .. }
            | Self::CommentRemoved { revision, .. }
            | Self::Cleared { revision }
            | Self::Paused { revision }
            | Self::Resumed { revision }
            | Self::SessionEnded { revision, .. } => *revision,
        }
    }
}

fn serialize_u64_as_string<S>(value: &u64, serializer: S) -> Result<S::Ok, S::Error>
where
    S: Serializer,
{
    serializer.serialize_str(&value.to_string())
}

fn deserialize_u64_from_string<'de, D>(deserializer: D) -> Result<u64, D::Error>
where
    D: Deserializer<'de>,
{
    let value = String::deserialize(deserializer)?;
    value.parse().map_err(D::Error::custom)
}

fn serialize_unix_ms_as_rfc3339<S>(value: &i64, serializer: S) -> Result<S::Ok, S::Error>
where
    S: Serializer,
{
    let timestamp = chrono::DateTime::<chrono::Utc>::from_timestamp_millis(*value)
        .ok_or_else(|| serde::ser::Error::custom("timestamp is outside the RFC 3339 range"))?;
    serializer.serialize_str(&timestamp.to_rfc3339_opts(chrono::SecondsFormat::Millis, true))
}

fn deserialize_unix_ms_from_rfc3339<'de, D>(deserializer: D) -> Result<i64, D::Error>
where
    D: Deserializer<'de>,
{
    let value = String::deserialize(deserializer)?;
    chrono::DateTime::parse_from_rfc3339(&value)
        .map(|timestamp| timestamp.timestamp_millis())
        .map_err(D::Error::custom)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn credentials_debug_output_redacts_capability() {
        let credentials = SessionCredentials {
            public_id: "public".to_string(),
            capability: "very-secret".to_string(),
        };

        let debug = format!("{credentials:?}");
        assert!(debug.contains("public"));
        assert!(debug.contains("[REDACTED]"));
        assert!(!debug.contains("very-secret"));
    }

    #[test]
    fn snapshot_event_serializes_as_a_flat_top_level_payload() {
        let event = OverlayEvent::Snapshot {
            snapshot: Snapshot {
                revision: 7,
                status: SessionStatus::Active,
                comments: Vec::new(),
                theme: OverlayTheme::Compact,
                show_avatar: false,
            },
        };

        let value = serde_json::to_value(event).unwrap();

        assert_eq!(value["type"], "snapshot");
        assert_eq!(value["revision"], 7);
        assert_eq!(value["status"], "active");
        assert_eq!(value["theme"], "compact");
        assert_eq!(value["show_avatar"], false);
        assert!(value.get("snapshot").is_none());
    }

    #[test]
    fn comment_ids_and_timestamps_use_browser_safe_json_formats() {
        let comment = OverlayComment {
            discord_message_id: 18_446_744_073_709_551_000,
            author_name: "Alice".to_string(),
            avatar_url: None,
            body: "hello".to_string(),
            created_at_unix_ms: 1_700_000_000_123,
            expires_at_unix_ms: 1_700_000_020_123,
        };

        let value = serde_json::to_value(&comment).unwrap();

        assert_eq!(value["discord_message_id"], "18446744073709551000");
        assert_eq!(value["created_at"], "2023-11-14T22:13:20.123Z");
        assert_eq!(value["expires_at"], "2023-11-14T22:13:40.123Z");
        assert!(value.get("created_at_unix_ms").is_none());
        assert!(value.get("expires_at_unix_ms").is_none());

        let decoded: OverlayComment = serde_json::from_value(value).unwrap();
        assert_eq!(decoded, comment);
    }

    #[test]
    fn removed_event_serializes_message_id_as_string() {
        let event = OverlayEvent::CommentRemoved {
            revision: 2,
            discord_message_id: 9_007_199_254_740_993,
        };

        let value = serde_json::to_value(event).unwrap();

        assert_eq!(value["discord_message_id"], "9007199254740993");
    }
}
