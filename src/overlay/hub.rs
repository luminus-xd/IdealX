use super::model::{
    HighlightedComment, NewComment, OverlayComment, OverlayEffectKind, OverlayEvent, OverlayTheme,
    SessionCredentials, SessionEndReason, SessionStatus, SessionSummary, Snapshot, StartSession,
};
use super::sanitize::{sanitize_author_name, sanitize_avatar_url, sanitize_body};
use rand::{rngs::OsRng, RngCore};
use sha2::{Digest, Sha256};
use std::collections::{HashMap, VecDeque};
use std::fmt;
use std::sync::Arc;
use std::time::Duration;
use subtle::ConstantTimeEq;
use tokio::sync::{broadcast, RwLock};
use tokio::time::Instant;

const MAX_COMMENTS: usize = 3;
const BROADCAST_CAPACITY: usize = 64;
const RATE_LIMIT_MAX_MESSAGES: usize = 5;
const RATE_LIMIT_WINDOW: Duration = Duration::from_secs(10);
const EFFECT_RATE_LIMIT_WINDOW: Duration = Duration::from_secs(2);

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum HubError {
    SessionAlreadyActive,
    SessionNotFound,
    Unauthorized,
    WrongChannel,
    AlreadyPaused,
    AlreadyActive,
}

impl fmt::Display for HubError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        let message = match self {
            Self::SessionAlreadyActive => "an overlay session is already active",
            Self::SessionNotFound => "no overlay session is active",
            Self::Unauthorized => "the user does not own this overlay session",
            Self::WrongChannel => "the command must be used in the overlay channel",
            Self::AlreadyPaused => "the overlay session is already paused",
            Self::AlreadyActive => "the overlay session is already active",
        };
        formatter.write_str(message)
    }
}

impl std::error::Error for HubError {}

/// 単一のインメモリOverlayセッションを管理するサービス。
pub struct OverlayHub {
    default_display_duration: Duration,
    session_ttl: Duration,
    session: RwLock<Option<Session>>,
}

struct Session {
    public_id: String,
    capability_hash: [u8; 32],
    guild_id: u64,
    channel_id: u64,
    owner_user_id: u64,
    status: SessionStatus,
    theme: OverlayTheme,
    show_avatar: bool,
    display_duration: Duration,
    revision: u64,
    expires_at: Instant,
    expires_at_unix_ms: i64,
    comments: VecDeque<StoredComment>,
    rate_limits: HashMap<u64, VecDeque<Instant>>,
    effect_rate_limits: HashMap<u64, Instant>,
    sender: broadcast::Sender<OverlayEvent>,
}

struct StoredComment {
    value: OverlayComment,
    expires_at: Instant,
}

impl OverlayHub {
    pub fn new(default_display_duration: Duration, session_ttl: Duration) -> Self {
        Self {
            default_display_duration,
            session_ttl,
            session: RwLock::new(None),
        }
    }

    pub async fn start(&self, request: StartSession) -> Result<SessionCredentials, HubError> {
        self.prune_expired().await;

        let mut guard = self.session.write().await;
        if guard.is_some() {
            return Err(HubError::SessionAlreadyActive);
        }

        let public_id = random_hex::<16>();
        let capability = random_hex::<32>();
        let capability_hash = hash_capability(&capability);
        let (sender, _) = broadcast::channel(BROADCAST_CAPACITY);
        let now = Instant::now();
        let now_unix_ms = unix_now_ms();
        let display_duration = if request.display_duration.is_zero() {
            self.default_display_duration
        } else {
            request.display_duration
        };

        *guard = Some(Session {
            public_id: public_id.clone(),
            capability_hash,
            guild_id: request.guild_id,
            channel_id: request.channel_id,
            owner_user_id: request.owner_user_id,
            status: SessionStatus::Active,
            theme: request.theme,
            show_avatar: request.show_avatar,
            display_duration,
            revision: 0,
            expires_at: now + self.session_ttl,
            expires_at_unix_ms: add_duration_ms(now_unix_ms, self.session_ttl),
            comments: VecDeque::with_capacity(MAX_COMMENTS),
            rate_limits: HashMap::new(),
            effect_rate_limits: HashMap::new(),
            sender,
        });

        Ok(SessionCredentials {
            public_id,
            capability,
        })
    }

    pub async fn status(&self) -> Option<SessionSummary> {
        self.prune_expired().await;
        let guard = self.session.read().await;
        guard.as_ref().map(Session::summary)
    }

    pub async fn add_comment(&self, input: NewComment) -> bool {
        self.prune_expired().await;
        let mut guard = self.session.write().await;
        let Some(session) = guard.as_mut() else {
            return false;
        };

        if session.status != SessionStatus::Active
            || session.guild_id != input.guild_id
            || session.channel_id != input.channel_id
            || session
                .comments
                .iter()
                .any(|comment| comment.value.discord_message_id == input.discord_message_id)
            || !session.accept_rate_limited_author(input.author_user_id)
        {
            return false;
        }

        let body = sanitize_body(&input.body);
        let author_name = sanitize_author_name(&input.author_name);
        if body.is_empty() || author_name.is_empty() {
            return false;
        }
        let keyword_effect = keyword_effect(&body);

        if session.comments.len() >= MAX_COMMENTS {
            if let Some(removed) = session.comments.pop_front() {
                session.bump_revision();
                session.send(OverlayEvent::CommentRemoved {
                    revision: session.revision,
                    discord_message_id: removed.value.discord_message_id,
                });
            }
        }

        let created_at_unix_ms = input.created_at_unix_ms.unwrap_or_else(unix_now_ms);
        let expires_at_unix_ms = add_duration_ms(unix_now_ms(), session.display_duration);
        let comment = OverlayComment {
            discord_message_id: input.discord_message_id,
            author_name,
            avatar_url: sanitize_avatar_url(input.avatar_url.as_deref()),
            body,
            created_at_unix_ms,
            expires_at_unix_ms,
        };

        session.comments.push_back(StoredComment {
            value: comment.clone(),
            expires_at: Instant::now() + session.display_duration,
        });
        session.bump_revision();
        session.send(OverlayEvent::CommentAdded {
            revision: session.revision,
            comment,
        });
        if let Some(effect) = keyword_effect {
            session.bump_revision();
            session.send(OverlayEvent::EffectTriggered {
                revision: session.revision,
                effect,
            });
        }
        true
    }

    /// 配信管理者が📌を付けたDiscordコメントを一時的に大きく表示する。
    pub async fn highlight_comment(
        &self,
        owner_user_id: u64,
        input: NewComment,
    ) -> Result<(), HubError> {
        self.prune_expired().await;
        let mut guard = self.session.write().await;
        let session = controlled_session(&mut guard, owner_user_id, input.channel_id)?;
        if session.status != SessionStatus::Active {
            return Err(HubError::AlreadyPaused);
        }
        if session.guild_id != input.guild_id {
            return Err(HubError::WrongChannel);
        }

        let body = sanitize_body(&input.body);
        let author_name = sanitize_author_name(&input.author_name);
        if body.is_empty() || author_name.is_empty() {
            return Ok(());
        }

        session.bump_revision();
        session.send(OverlayEvent::CommentHighlighted {
            revision: session.revision,
            highlight: HighlightedComment {
                discord_message_id: input.discord_message_id,
                author_name,
                avatar_url: sanitize_avatar_url(input.avatar_url.as_deref()),
                body,
            },
        });
        Ok(())
    }

    /// ✌️・🤟リアクションに対応する一過性の演出を配信する。
    pub async fn trigger_effect(
        &self,
        channel_id: u64,
        author_user_id: u64,
        effect: OverlayEffectKind,
    ) -> bool {
        self.prune_expired().await;
        let mut guard = self.session.write().await;
        let Some(session) = guard.as_mut() else {
            return false;
        };
        if session.status != SessionStatus::Active
            || session.channel_id != channel_id
            || !session.accept_effect_author(author_user_id)
        {
            return false;
        }

        session.bump_revision();
        session.send(OverlayEvent::EffectTriggered {
            revision: session.revision,
            effect,
        });
        true
    }

    pub async fn update_comment(&self, channel_id: u64, message_id: u64, body: &str) -> bool {
        self.prune_expired().await;
        let sanitized = sanitize_body(body);
        if sanitized.is_empty() {
            return self.remove_comment(channel_id, message_id).await;
        }

        let mut guard = self.session.write().await;
        let Some(session) = guard.as_mut() else {
            return false;
        };
        if session.channel_id != channel_id {
            return false;
        }

        let Some(index) = session
            .comments
            .iter()
            .position(|comment| comment.value.discord_message_id == message_id)
        else {
            return false;
        };

        session.comments[index].value.body = sanitized;
        let updated = session.comments[index].value.clone();
        session.bump_revision();
        session.send(OverlayEvent::CommentUpdated {
            revision: session.revision,
            comment: updated,
        });
        true
    }

    pub async fn remove_comment(&self, channel_id: u64, message_id: u64) -> bool {
        self.prune_expired().await;
        let mut guard = self.session.write().await;
        let Some(session) = guard.as_mut() else {
            return false;
        };
        if session.channel_id != channel_id {
            return false;
        }

        let Some(index) = session
            .comments
            .iter()
            .position(|comment| comment.value.discord_message_id == message_id)
        else {
            return false;
        };

        session.comments.remove(index);
        session.bump_revision();
        session.send(OverlayEvent::CommentRemoved {
            revision: session.revision,
            discord_message_id: message_id,
        });
        true
    }

    pub async fn remove_comments(&self, channel_id: u64, message_ids: &[u64]) -> usize {
        self.prune_expired().await;
        let mut guard = self.session.write().await;
        let Some(session) = guard.as_mut() else {
            return 0;
        };
        if session.channel_id != channel_id {
            return 0;
        }

        let mut removed = 0;
        for message_id in message_ids {
            let Some(index) = session
                .comments
                .iter()
                .position(|comment| comment.value.discord_message_id == *message_id)
            else {
                continue;
            };

            session.comments.remove(index);
            session.bump_revision();
            session.send(OverlayEvent::CommentRemoved {
                revision: session.revision,
                discord_message_id: *message_id,
            });
            removed += 1;
        }
        removed
    }

    pub async fn pause(&self, owner_user_id: u64, channel_id: u64) -> Result<(), HubError> {
        self.prune_expired().await;
        let mut guard = self.session.write().await;
        let session = controlled_session(&mut guard, owner_user_id, channel_id)?;
        if session.status == SessionStatus::Paused {
            return Err(HubError::AlreadyPaused);
        }

        session.comments.clear();
        session.status = SessionStatus::Paused;
        session.bump_revision();
        session.send(OverlayEvent::Paused {
            revision: session.revision,
        });
        Ok(())
    }

    pub async fn resume(&self, owner_user_id: u64, channel_id: u64) -> Result<(), HubError> {
        self.prune_expired().await;
        let mut guard = self.session.write().await;
        let session = controlled_session(&mut guard, owner_user_id, channel_id)?;
        if session.status == SessionStatus::Active {
            return Err(HubError::AlreadyActive);
        }

        session.status = SessionStatus::Active;
        session.bump_revision();
        session.send(OverlayEvent::Resumed {
            revision: session.revision,
        });
        Ok(())
    }

    pub async fn clear(&self, owner_user_id: u64, channel_id: u64) -> Result<(), HubError> {
        self.prune_expired().await;
        let mut guard = self.session.write().await;
        let session = controlled_session(&mut guard, owner_user_id, channel_id)?;
        session.comments.clear();
        session.bump_revision();
        session.send(OverlayEvent::Cleared {
            revision: session.revision,
        });
        Ok(())
    }

    pub async fn end(&self, owner_user_id: u64, channel_id: u64) -> Result<(), HubError> {
        self.prune_expired().await;
        let mut guard = self.session.write().await;
        let session = controlled_session(&mut guard, owner_user_id, channel_id)?;
        session.bump_revision();
        session.send(OverlayEvent::SessionEnded {
            revision: session.revision,
            reason: SessionEndReason::Ended,
        });
        *guard = None;
        Ok(())
    }

    pub async fn rotate(
        &self,
        owner_user_id: u64,
        channel_id: u64,
    ) -> Result<SessionCredentials, HubError> {
        self.prune_expired().await;
        let mut guard = self.session.write().await;
        let session = controlled_session(&mut guard, owner_user_id, channel_id)?;

        let capability = random_hex::<32>();
        session.capability_hash = hash_capability(&capability);
        session.bump_revision();
        session.send(OverlayEvent::SessionEnded {
            revision: session.revision,
            reason: SessionEndReason::CapabilityRotated,
        });
        let (sender, _) = broadcast::channel(BROADCAST_CAPACITY);
        session.sender = sender;

        Ok(SessionCredentials {
            public_id: session.public_id.clone(),
            capability,
        })
    }

    pub async fn clear_for_sync_loss(&self) {
        let mut guard = self.session.write().await;
        let Some(session) = guard.as_mut() else {
            return;
        };

        session.comments.clear();
        session.bump_revision();
        session.send(OverlayEvent::Cleared {
            revision: session.revision,
        });
    }

    pub async fn authenticate(
        &self,
        public_id: &str,
        capability: &str,
    ) -> Option<(Snapshot, broadcast::Receiver<OverlayEvent>)> {
        self.prune_expired().await;
        let guard = self.session.read().await;
        let session = guard.as_ref()?;
        if session.public_id != public_id
            || !constant_time_eq(&session.capability_hash, &hash_capability(capability))
        {
            return None;
        }

        Some((session.snapshot(), session.sender.subscribe()))
    }

    pub async fn has_public_id(&self, public_id: &str) -> bool {
        self.prune_expired().await;
        let guard = self.session.read().await;
        guard
            .as_ref()
            .is_some_and(|session| session.public_id == public_id)
    }

    /// コメント期限とセッションTTLを適用し、必要なイベントをbroadcastする。
    pub async fn prune_expired(&self) {
        let mut guard = self.session.write().await;
        let Some(session) = guard.as_mut() else {
            return;
        };

        let now = Instant::now();
        if now >= session.expires_at {
            session.bump_revision();
            session.send(OverlayEvent::SessionEnded {
                revision: session.revision,
                reason: SessionEndReason::Expired,
            });
            *guard = None;
            return;
        }

        let expired_ids = session
            .comments
            .iter()
            .filter(|comment| now >= comment.expires_at)
            .map(|comment| comment.value.discord_message_id)
            .collect::<Vec<_>>();

        for message_id in expired_ids {
            if let Some(index) = session
                .comments
                .iter()
                .position(|comment| comment.value.discord_message_id == message_id)
            {
                session.comments.remove(index);
                session.bump_revision();
                session.send(OverlayEvent::CommentRemoved {
                    revision: session.revision,
                    discord_message_id: message_id,
                });
            }
        }
    }

    /// 呼び出し側がspawnしておくことで、投稿が来ない間も期限切れを反映できる。
    pub async fn run_expiration_loop(self: Arc<Self>) {
        let mut interval = tokio::time::interval(Duration::from_millis(250));
        interval.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Skip);
        loop {
            interval.tick().await;
            self.prune_expired().await;
        }
    }
}

impl Session {
    fn summary(&self) -> SessionSummary {
        SessionSummary {
            public_id: self.public_id.clone(),
            guild_id: self.guild_id,
            channel_id: self.channel_id,
            owner_user_id: self.owner_user_id,
            status: self.status,
            revision: self.revision,
            comment_count: self.comments.len(),
            expires_at_unix_ms: self.expires_at_unix_ms,
            theme: self.theme,
            show_avatar: self.show_avatar,
        }
    }

    fn snapshot(&self) -> Snapshot {
        Snapshot {
            revision: self.revision,
            status: self.status,
            comments: self
                .comments
                .iter()
                .map(|comment| comment.value.clone())
                .collect(),
            theme: self.theme,
            show_avatar: self.show_avatar,
        }
    }

    fn bump_revision(&mut self) {
        self.revision = self.revision.saturating_add(1);
    }

    fn send(&self, event: OverlayEvent) {
        let _ = self.sender.send(event);
    }

    fn accept_rate_limited_author(&mut self, author_user_id: u64) -> bool {
        let now = Instant::now();
        let entries = self.rate_limits.entry(author_user_id).or_default();
        while entries
            .front()
            .is_some_and(|seen_at| now.duration_since(*seen_at) >= RATE_LIMIT_WINDOW)
        {
            entries.pop_front();
        }

        if entries.len() >= RATE_LIMIT_MAX_MESSAGES {
            return false;
        }
        entries.push_back(now);
        true
    }

    fn accept_effect_author(&mut self, author_user_id: u64) -> bool {
        let now = Instant::now();
        if self
            .effect_rate_limits
            .get(&author_user_id)
            .is_some_and(|seen_at| now.duration_since(*seen_at) < EFFECT_RATE_LIMIT_WINDOW)
        {
            return false;
        }
        self.effect_rate_limits.insert(author_user_id, now);
        true
    }
}

fn controlled_session(
    guard: &mut Option<Session>,
    owner_user_id: u64,
    channel_id: u64,
) -> Result<&mut Session, HubError> {
    let session = guard.as_mut().ok_or(HubError::SessionNotFound)?;
    if session.owner_user_id != owner_user_id {
        return Err(HubError::Unauthorized);
    }
    if session.channel_id != channel_id {
        return Err(HubError::WrongChannel);
    }
    Ok(session)
}

fn random_hex<const N: usize>() -> String {
    let mut bytes = [0_u8; N];
    OsRng.fill_bytes(&mut bytes);
    bytes.iter().map(|byte| format!("{byte:02x}")).collect()
}

fn hash_capability(capability: &str) -> [u8; 32] {
    Sha256::digest(capability.as_bytes()).into()
}

fn constant_time_eq(left: &[u8; 32], right: &[u8; 32]) -> bool {
    bool::from(left.ct_eq(right))
}

fn unix_now_ms() -> i64 {
    chrono::Utc::now().timestamp_millis()
}

fn add_duration_ms(base: i64, duration: Duration) -> i64 {
    base.saturating_add(i64::try_from(duration.as_millis()).unwrap_or(i64::MAX))
}

fn keyword_effect(body: &str) -> Option<OverlayEffectKind> {
    if contains_ascii_word(body, "gg") {
        Some(OverlayEffectKind::GoodGame)
    } else if body.contains('草') {
        Some(OverlayEffectKind::Grass)
    } else if body.to_ascii_lowercase().contains("www") {
        Some(OverlayEffectKind::Laugh)
    } else {
        None
    }
}

fn contains_ascii_word(body: &str, expected: &str) -> bool {
    body.split(|character: char| !character.is_ascii_alphanumeric() && character != '_')
        .any(|word| word.eq_ignore_ascii_case(expected))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn hub() -> OverlayHub {
        OverlayHub::new(Duration::from_secs(20), Duration::from_secs(60))
    }

    fn start_request() -> StartSession {
        StartSession {
            guild_id: 1,
            channel_id: 2,
            owner_user_id: 3,
            theme: OverlayTheme::Dark,
            show_avatar: true,
            display_duration: Duration::from_secs(20),
        }
    }

    fn comment(message_id: u64, author_id: u64) -> NewComment {
        NewComment {
            guild_id: 1,
            channel_id: 2,
            discord_message_id: message_id,
            author_user_id: author_id,
            author_name: "Alice".to_string(),
            avatar_url: None,
            body: format!("comment {message_id}"),
            created_at_unix_ms: Some(1_000),
        }
    }

    #[tokio::test]
    async fn only_one_session_can_be_started() {
        let hub = hub();
        hub.start(start_request()).await.unwrap();

        assert_eq!(
            hub.start(start_request()).await,
            Err(HubError::SessionAlreadyActive)
        );
    }

    #[tokio::test]
    async fn credentials_authenticate_and_wrong_capability_does_not() {
        let hub = hub();
        let credentials = hub.start(start_request()).await.unwrap();

        assert!(hub
            .authenticate(&credentials.public_id, &credentials.capability)
            .await
            .is_some());
        assert!(hub
            .authenticate(&credentials.public_id, "wrong")
            .await
            .is_none());
    }

    #[tokio::test]
    async fn fourth_comment_evicts_the_oldest() {
        let hub = hub();
        let credentials = hub.start(start_request()).await.unwrap();
        for message_id in 1..=4 {
            assert!(hub.add_comment(comment(message_id, message_id)).await);
        }

        let (snapshot, _) = hub
            .authenticate(&credentials.public_id, &credentials.capability)
            .await
            .unwrap();
        let ids = snapshot
            .comments
            .iter()
            .map(|comment| comment.discord_message_id)
            .collect::<Vec<_>>();
        assert_eq!(ids, vec![2, 3, 4]);
    }

    #[tokio::test]
    async fn pause_clears_and_blocks_new_comments_until_resume() {
        let hub = hub();
        let credentials = hub.start(start_request()).await.unwrap();
        assert!(hub.add_comment(comment(1, 1)).await);

        hub.pause(3, 2).await.unwrap();
        assert!(!hub.add_comment(comment(2, 2)).await);

        let (paused, _) = hub
            .authenticate(&credentials.public_id, &credentials.capability)
            .await
            .unwrap();
        assert_eq!(paused.status, SessionStatus::Paused);
        assert!(paused.comments.is_empty());

        hub.resume(3, 2).await.unwrap();
        assert!(hub.add_comment(comment(3, 3)).await);
    }

    #[tokio::test]
    async fn only_owner_in_session_channel_can_control() {
        let hub = hub();
        hub.start(start_request()).await.unwrap();

        assert_eq!(hub.pause(999, 2).await, Err(HubError::Unauthorized));
        assert_eq!(hub.pause(3, 999).await, Err(HubError::WrongChannel));
    }

    #[tokio::test]
    async fn edit_and_delete_are_broadcast_with_increasing_revisions() {
        let hub = hub();
        let credentials = hub.start(start_request()).await.unwrap();
        let (_, mut receiver) = hub
            .authenticate(&credentials.public_id, &credentials.capability)
            .await
            .unwrap();

        assert!(hub.add_comment(comment(1, 1)).await);
        let added = receiver.recv().await.unwrap();
        assert!(hub.update_comment(2, 1, "updated").await);
        let updated = receiver.recv().await.unwrap();
        assert!(hub.remove_comment(2, 1).await);
        let removed = receiver.recv().await.unwrap();

        assert!(added.revision() < updated.revision());
        assert!(updated.revision() < removed.revision());
    }

    #[tokio::test]
    async fn rotate_invalidates_old_capability() {
        let hub = hub();
        let original = hub.start(start_request()).await.unwrap();

        let rotated = hub.rotate(3, 2).await.unwrap();

        assert_eq!(rotated.public_id, original.public_id);
        assert!(hub
            .authenticate(&original.public_id, &original.capability)
            .await
            .is_none());
        assert!(hub
            .authenticate(&rotated.public_id, &rotated.capability)
            .await
            .is_some());
    }

    #[tokio::test]
    async fn end_removes_session_and_invalidates_url() {
        let hub = hub();
        let credentials = hub.start(start_request()).await.unwrap();

        hub.end(3, 2).await.unwrap();

        assert!(hub.status().await.is_none());
        assert!(!hub.has_public_id(&credentials.public_id).await);
    }

    #[tokio::test]
    async fn immediate_rate_limit_is_enforced_per_author() {
        let hub = hub();
        hub.start(start_request()).await.unwrap();

        for message_id in 1..=RATE_LIMIT_MAX_MESSAGES as u64 {
            assert!(hub.add_comment(comment(message_id, 10)).await);
        }
        assert!(!hub.add_comment(comment(99, 10)).await);
        assert!(hub.add_comment(comment(100, 11)).await);
    }

    #[tokio::test]
    async fn owner_can_highlight_a_comment_in_the_session_channel() {
        let hub = hub();
        let credentials = hub.start(start_request()).await.unwrap();
        let (_, mut receiver) = hub
            .authenticate(&credentials.public_id, &credentials.capability)
            .await
            .unwrap();

        hub.highlight_comment(3, comment(1, 10)).await.unwrap();
        let event = receiver.recv().await.unwrap();

        assert!(matches!(
            event,
            OverlayEvent::CommentHighlighted { highlight, .. }
                if highlight.discord_message_id == 1
        ));
        assert_eq!(
            hub.highlight_comment(999, comment(2, 10)).await,
            Err(HubError::Unauthorized)
        );
    }

    #[tokio::test]
    async fn reaction_effects_are_rate_limited_per_author() {
        let hub = hub();
        let credentials = hub.start(start_request()).await.unwrap();
        let (_, mut receiver) = hub
            .authenticate(&credentials.public_id, &credentials.capability)
            .await
            .unwrap();

        assert!(hub.trigger_effect(2, 10, OverlayEffectKind::Peace).await);
        assert!(!hub.trigger_effect(2, 10, OverlayEffectKind::RockOn).await);
        assert!(hub.trigger_effect(2, 11, OverlayEffectKind::RockOn).await);

        assert!(matches!(
            receiver.recv().await.unwrap(),
            OverlayEvent::EffectTriggered {
                effect: OverlayEffectKind::Peace,
                ..
            }
        ));
        assert!(matches!(
            receiver.recv().await.unwrap(),
            OverlayEvent::EffectTriggered {
                effect: OverlayEffectKind::RockOn,
                ..
            }
        ));
    }

    #[tokio::test]
    async fn accepted_keyword_comment_triggers_one_effect() {
        let hub = hub();
        let credentials = hub.start(start_request()).await.unwrap();
        let (_, mut receiver) = hub
            .authenticate(&credentials.public_id, &credentials.capability)
            .await
            .unwrap();
        let mut input = comment(1, 10);
        input.body = "GG 草 www".to_string();

        assert!(hub.add_comment(input).await);
        assert!(matches!(
            receiver.recv().await.unwrap(),
            OverlayEvent::CommentAdded { .. }
        ));
        assert!(matches!(
            receiver.recv().await.unwrap(),
            OverlayEvent::EffectTriggered {
                effect: OverlayEffectKind::GoodGame,
                ..
            }
        ));
        assert!(receiver.try_recv().is_err());
    }

    #[test]
    fn keyword_detection_is_case_insensitive_and_avoids_partial_gg() {
        assert_eq!(keyword_effect("gg!"), Some(OverlayEffectKind::GoodGame));
        assert_eq!(keyword_effect("これは草"), Some(OverlayEffectKind::Grass));
        assert_eq!(keyword_effect("wwww"), Some(OverlayEffectKind::Laugh));
        assert_eq!(keyword_effect("egg"), None);
    }

    #[tokio::test]
    async fn expired_comments_are_removed() {
        let hub = OverlayHub::new(Duration::from_millis(1), Duration::from_secs(60));
        let mut request = start_request();
        request.display_duration = Duration::ZERO;
        let credentials = hub.start(request).await.unwrap();
        assert!(hub.add_comment(comment(1, 1)).await);

        tokio::time::sleep(Duration::from_millis(5)).await;
        hub.prune_expired().await;

        let (snapshot, _) = hub
            .authenticate(&credentials.public_id, &credentials.capability)
            .await
            .unwrap();
        assert!(snapshot.comments.is_empty());
    }

    #[tokio::test]
    async fn expired_session_is_removed() {
        let hub = OverlayHub::new(Duration::from_secs(20), Duration::from_millis(1));
        hub.start(start_request()).await.unwrap();

        tokio::time::sleep(Duration::from_millis(5)).await;
        hub.prune_expired().await;

        assert!(hub.status().await.is_none());
    }
}
