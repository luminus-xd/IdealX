use std::{
    collections::HashMap,
    sync::{
        atomic::{AtomicU64, Ordering},
        Arc, Mutex,
    },
    time::{Duration, Instant},
};

use rand::{rngs::OsRng, RngCore};

/// 1ページあたりの HTML の上限
pub const MAX_HTML_BYTES: usize = 512 * 1024;
/// タイトルの上限（文字数）
pub const MAX_TITLE_CHARS: usize = 100;
/// 同時に保持するページ数の上限
const MAX_ENTRIES: usize = 200;
/// 保持する HTML の合計サイズの上限
const MAX_TOTAL_BYTES: usize = 64 * 1024 * 1024;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ArtifactLink {
    pub id: String,
    pub title: String,
    pub url: String,
}

#[derive(Debug, PartialEq, Eq)]
pub enum ArtifactError {
    EmptyTitle,
    TitleTooLong,
    EmptyHtml,
    HtmlTooLarge,
}

impl std::fmt::Display for ArtifactError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::EmptyTitle => write!(f, "title が空です"),
            Self::TitleTooLong => write!(f, "title は {MAX_TITLE_CHARS} 文字以内にしてください"),
            Self::EmptyHtml => write!(f, "html が空です"),
            Self::HtmlTooLarge => write!(
                f,
                "html が大きすぎます（上限 {} KiB）",
                MAX_HTML_BYTES / 1024
            ),
        }
    }
}

impl std::error::Error for ArtifactError {}

struct Entry {
    html: Arc<str>,
    created_at: Instant,
    /// 作成順。Instant は同値になりうるので、古い順の判定にはこちらを使う
    seq: u64,
}

/// 再起動で消えるメモリ上のアーティファクト置き場
pub struct ArtifactStore {
    public_base_url: String,
    ttl: Duration,
    entries: Mutex<HashMap<String, Entry>>,
    next_seq: AtomicU64,
}

impl ArtifactStore {
    pub fn new(public_base_url: impl Into<String>, ttl: Duration) -> Self {
        Self {
            public_base_url: public_base_url.into().trim_end_matches('/').to_string(),
            ttl,
            entries: Mutex::new(HashMap::new()),
            next_seq: AtomicU64::new(0),
        }
    }

    pub fn ttl(&self) -> Duration {
        self.ttl
    }

    pub fn create(&self, title: &str, html: &str) -> Result<ArtifactLink, ArtifactError> {
        let title = title.trim();
        if title.is_empty() {
            return Err(ArtifactError::EmptyTitle);
        }
        if title.chars().count() > MAX_TITLE_CHARS {
            return Err(ArtifactError::TitleTooLong);
        }
        if html.trim().is_empty() {
            return Err(ArtifactError::EmptyHtml);
        }
        if html.len() > MAX_HTML_BYTES {
            return Err(ArtifactError::HtmlTooLarge);
        }

        let id = random_hex::<16>();
        let now = Instant::now();
        let mut entries = self.lock();
        self.evict(&mut entries, now, html.len());
        entries.insert(
            id.clone(),
            Entry {
                html: Arc::from(html),
                created_at: now,
                seq: self.next_seq.fetch_add(1, Ordering::Relaxed),
            },
        );

        Ok(ArtifactLink {
            url: format!("{}/a/{id}", self.public_base_url),
            id,
            title: title.to_string(),
        })
    }

    pub fn get(&self, id: &str) -> Option<Arc<str>> {
        let entries = self.lock();
        entries
            .get(id)
            .filter(|entry| entry.created_at.elapsed() < self.ttl)
            .map(|entry| entry.html.clone())
    }

    /// 期限切れを捨て、上限を超えるなら古い順に捨てて `incoming` バイト分の空きを作る
    fn evict(&self, entries: &mut HashMap<String, Entry>, now: Instant, incoming: usize) {
        entries.retain(|_, entry| now.duration_since(entry.created_at) < self.ttl);

        let mut total: usize = entries.values().map(|e| e.html.len()).sum();
        while !entries.is_empty()
            && (entries.len() >= MAX_ENTRIES || total + incoming > MAX_TOTAL_BYTES)
        {
            let Some(oldest) = entries
                .iter()
                .min_by_key(|(_, entry)| entry.seq)
                .map(|(id, _)| id.clone())
            else {
                break;
            };
            if let Some(removed) = entries.remove(&oldest) {
                total -= removed.html.len();
            }
        }
    }

    fn lock(&self) -> std::sync::MutexGuard<'_, HashMap<String, Entry>> {
        // 保持しているのは読み取り専用のデータだけなので、poison されても中身はそのまま使える
        self.entries
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
    }
}

fn random_hex<const N: usize>() -> String {
    let mut bytes = [0_u8; N];
    OsRng.fill_bytes(&mut bytes);
    bytes.iter().map(|byte| format!("{byte:02x}")).collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn store() -> ArtifactStore {
        ArtifactStore::new("https://bot.example.com/", Duration::from_secs(60))
    }

    #[test]
    fn create_returns_unguessable_public_url() {
        let store = store();
        let link = store.create("  グラフ  ", "<h1>hi</h1>").unwrap();

        assert_eq!(link.id.len(), 32);
        assert_eq!(link.title, "グラフ");
        assert_eq!(link.url, format!("https://bot.example.com/a/{}", link.id));
        assert_eq!(&*store.get(&link.id).unwrap(), "<h1>hi</h1>");
    }

    #[test]
    fn rejects_invalid_input() {
        let store = store();
        assert_eq!(
            store.create(" ", "<p>x</p>"),
            Err(ArtifactError::EmptyTitle)
        );
        assert_eq!(store.create("t", "  "), Err(ArtifactError::EmptyHtml));
        assert_eq!(
            store.create(&"あ".repeat(MAX_TITLE_CHARS + 1), "<p>x</p>"),
            Err(ArtifactError::TitleTooLong)
        );
        assert_eq!(
            store.create("t", &"a".repeat(MAX_HTML_BYTES + 1)),
            Err(ArtifactError::HtmlTooLarge)
        );
    }

    #[test]
    fn expired_artifacts_are_not_served() {
        let store = ArtifactStore::new("https://bot.example.com", Duration::ZERO);
        let link = store.create("t", "<p>x</p>").unwrap();
        assert!(store.get(&link.id).is_none());
    }

    #[test]
    fn oldest_artifact_is_evicted_when_full() {
        let store = store();
        let first = store.create("first", "<p>1</p>").unwrap();
        for _ in 0..MAX_ENTRIES {
            store.create("t", "<p>x</p>").unwrap();
        }
        assert!(store.get(&first.id).is_none());
        assert_eq!(store.lock().len(), MAX_ENTRIES);
    }
}
