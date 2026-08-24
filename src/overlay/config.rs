use std::{collections::HashSet, env, net::SocketAddr};

use anyhow::{bail, Context as _};
use url::Url;

const DEFAULT_DISPLAY_SECONDS: u64 = 20;
const DEFAULT_SESSION_TTL_MINUTES: u64 = 480;

#[derive(Clone, Debug)]
pub struct OverlayConfig {
    pub enabled: bool,
    pub public_base_url: String,
    pub allowed_owner_ids: HashSet<u64>,
    pub default_display_seconds: u64,
    pub session_ttl_minutes: u64,
    pub listen_addr: SocketAddr,
}

impl OverlayConfig {
    pub fn from_env() -> anyhow::Result<Self> {
        let enabled = env::var("OVERLAY_ENABLED")
            .ok()
            .map(|value| {
                matches!(
                    value.trim().to_ascii_lowercase().as_str(),
                    "1" | "true" | "yes" | "on"
                )
            })
            .unwrap_or(false);

        let public_base_url = env::var("OVERLAY_PUBLIC_BASE_URL")
            .unwrap_or_default()
            .trim_end_matches('/')
            .to_string();
        let allowed_owner_ids = env::var("OVERLAY_ALLOWED_OWNER_IDS")
            .unwrap_or_default()
            .split(',')
            .filter_map(|id| id.trim().parse().ok())
            .collect::<HashSet<_>>();
        let default_display_seconds =
            parse_u64_env("OVERLAY_DEFAULT_DISPLAY_SECONDS", DEFAULT_DISPLAY_SECONDS)?;
        if !matches!(default_display_seconds, 10 | 20 | 30 | 60) {
            bail!("OVERLAY_DEFAULT_DISPLAY_SECONDS must be one of 10, 20, 30, or 60");
        }

        let session_ttl_minutes =
            parse_u64_env("OVERLAY_SESSION_TTL_MINUTES", DEFAULT_SESSION_TTL_MINUTES)?;
        if session_ttl_minutes == 0 {
            bail!("OVERLAY_SESSION_TTL_MINUTES must be greater than zero");
        }

        let port = env::var("PORT")
            .unwrap_or_else(|_| "3000".to_string())
            .parse::<u16>()
            .context("PORT must be a valid TCP port")?;

        if enabled {
            if public_base_url.is_empty() {
                bail!("OVERLAY_PUBLIC_BASE_URL is required when OVERLAY_ENABLED is true");
            }
            let url = Url::parse(&public_base_url)
                .context("OVERLAY_PUBLIC_BASE_URL must be a valid HTTPS URL")?;
            if url.scheme() != "https" || url.host_str().is_none() {
                bail!("OVERLAY_PUBLIC_BASE_URL must use HTTPS and include a host");
            }
            if !url.username().is_empty()
                || url.password().is_some()
                || url.query().is_some()
                || url.fragment().is_some()
                || url.path() != "/"
            {
                bail!(
                    "OVERLAY_PUBLIC_BASE_URL must be an origin without credentials, path, query, or fragment"
                );
            }
            if allowed_owner_ids.is_empty() {
                bail!("OVERLAY_ALLOWED_OWNER_IDS must contain at least one user ID when overlay is enabled");
            }
        }

        Ok(Self {
            enabled,
            public_base_url,
            allowed_owner_ids,
            default_display_seconds,
            session_ttl_minutes,
            listen_addr: SocketAddr::from(([0, 0, 0, 0], port)),
        })
    }

    pub fn owner_is_allowed(&self, user_id: u64) -> bool {
        self.allowed_owner_ids.contains(&user_id)
    }
}

fn parse_u64_env(name: &str, default: u64) -> anyhow::Result<u64> {
    match env::var(name) {
        Ok(value) => value
            .parse::<u64>()
            .with_context(|| format!("{name} must be a positive integer")),
        Err(_) => Ok(default),
    }
}
