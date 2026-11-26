use std::env;

use anyhow::{bail, Context as _};
use url::Url;

const DEFAULT_TTL_HOURS: u64 = 24;

#[derive(Clone, Debug)]
pub struct ArtifactConfig {
    pub enabled: bool,
    pub public_base_url: String,
    pub ttl_hours: u64,
}

impl ArtifactConfig {
    pub fn from_env() -> anyhow::Result<Self> {
        let enabled = env::var("ARTIFACT_ENABLED")
            .ok()
            .map(|value| {
                matches!(
                    value.trim().to_ascii_lowercase().as_str(),
                    "1" | "true" | "yes" | "on"
                )
            })
            .unwrap_or(false);

        // 専用の値がなければオーバーレイと同じ公開オリジンを使う
        let public_base_url = env::var("ARTIFACT_PUBLIC_BASE_URL")
            .or_else(|_| env::var("OVERLAY_PUBLIC_BASE_URL"))
            .unwrap_or_default()
            .trim_end_matches('/')
            .to_string();

        let ttl_hours = match env::var("ARTIFACT_TTL_HOURS") {
            Ok(value) => value
                .parse::<u64>()
                .context("ARTIFACT_TTL_HOURS must be a positive integer")?,
            Err(_) => DEFAULT_TTL_HOURS,
        };
        if ttl_hours == 0 {
            bail!("ARTIFACT_TTL_HOURS must be greater than zero");
        }

        if enabled {
            if public_base_url.is_empty() {
                bail!("ARTIFACT_PUBLIC_BASE_URL (or OVERLAY_PUBLIC_BASE_URL) is required when ARTIFACT_ENABLED is true");
            }
            let url = Url::parse(&public_base_url)
                .context("ARTIFACT_PUBLIC_BASE_URL must be a valid HTTPS URL")?;
            if url.scheme() != "https" || url.host_str().is_none() {
                bail!("ARTIFACT_PUBLIC_BASE_URL must use HTTPS and include a host");
            }
            if !url.username().is_empty()
                || url.password().is_some()
                || url.query().is_some()
                || url.fragment().is_some()
                || url.path() != "/"
            {
                bail!(
                    "ARTIFACT_PUBLIC_BASE_URL must be an origin without credentials, path, query, or fragment"
                );
            }
        }

        Ok(Self {
            enabled,
            public_base_url,
            ttl_hours,
        })
    }
}
