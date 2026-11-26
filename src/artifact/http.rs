use std::sync::Arc;

use axum::{
    extract::{Path, State},
    http::{header, HeaderName, HeaderValue, StatusCode},
    response::{IntoResponse, Response},
    routing::get,
    Router,
};

use super::store::ArtifactStore;

/// LLM が書いた HTML を、bot と同じドメインのまま安全に表示するための CSP。
///
/// `sandbox` ディレクティブ（`allow-same-origin` なし）でページを不透明オリジンに閉じ込めるので、
/// スクリプトはこのドメインの Cookie・ストレージ・他ページに触れられない。
/// `connect-src 'none'` で fetch / XHR / WebSocket による外部送信も止める。
const ARTIFACT_CSP: &str = concat!(
    "sandbox allow-scripts allow-modals allow-popups allow-popups-to-escape-sandbox allow-downloads; ",
    "default-src 'none'; ",
    "script-src 'unsafe-inline' 'unsafe-eval' https://cdnjs.cloudflare.com https://cdn.jsdelivr.net https://unpkg.com; ",
    "style-src 'unsafe-inline' https://cdnjs.cloudflare.com https://cdn.jsdelivr.net https://unpkg.com https://fonts.googleapis.com; ",
    "font-src data: https://cdnjs.cloudflare.com https://cdn.jsdelivr.net https://fonts.gstatic.com; ",
    "img-src data: blob: https:; ",
    "media-src data: blob:; ",
    "connect-src 'none'; ",
    "frame-src 'none'; ",
    "worker-src blob:; ",
    "form-action 'none'; ",
    "base-uri 'none'; ",
    "frame-ancestors 'none'"
);

pub fn router(store: Arc<ArtifactStore>) -> Router {
    Router::new()
        .route("/a/{id}", get(artifact_page))
        .with_state(store)
}

async fn artifact_page(
    State(store): State<Arc<ArtifactStore>>,
    Path(id): Path<String>,
) -> Response {
    let Some(html) = store.get(&id) else {
        return (
            StatusCode::NOT_FOUND,
            [(header::CONTENT_TYPE, "text/plain; charset=utf-8")],
            "このページは見つからないか、有効期限が切れました。",
        )
            .into_response();
    };

    let mut response = (
        [(header::CONTENT_TYPE, "text/html; charset=utf-8")],
        html.to_string(),
    )
        .into_response();

    let headers = response.headers_mut();
    headers.insert(
        header::CONTENT_SECURITY_POLICY,
        HeaderValue::from_static(ARTIFACT_CSP),
    );
    headers.insert(
        header::CACHE_CONTROL,
        HeaderValue::from_static("no-store, private"),
    );
    headers.insert(
        header::REFERRER_POLICY,
        HeaderValue::from_static("no-referrer"),
    );
    headers.insert(
        header::X_CONTENT_TYPE_OPTIONS,
        HeaderValue::from_static("nosniff"),
    );
    headers.insert(
        HeaderName::from_static("x-robots-tag"),
        HeaderValue::from_static("noindex, nofollow"),
    );
    headers.insert(
        HeaderName::from_static("permissions-policy"),
        HeaderValue::from_static(
            "camera=(), microphone=(), geolocation=(), usb=(), payment=(), display-capture=()",
        ),
    );
    headers.insert(
        HeaderName::from_static("cross-origin-opener-policy"),
        HeaderValue::from_static("same-origin"),
    );
    response
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::{
        body::{to_bytes, Body},
        http::Request,
    };
    use std::time::Duration;
    use tower::ServiceExt;

    fn store() -> Arc<ArtifactStore> {
        Arc::new(ArtifactStore::new(
            "https://bot.example.com",
            Duration::from_secs(60),
        ))
    }

    #[tokio::test]
    async fn serves_html_inside_csp_sandbox() {
        let store = store();
        let link = store.create("t", "<h1>hello</h1>").unwrap();

        let response = router(store)
            .oneshot(
                Request::builder()
                    .uri(format!("/a/{}", link.id))
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::OK);
        let csp = response.headers()[header::CONTENT_SECURITY_POLICY]
            .to_str()
            .unwrap()
            .to_string();
        assert!(csp.starts_with("sandbox allow-scripts "));
        assert!(!csp.contains("allow-same-origin"));
        assert!(csp.contains("connect-src 'none'"));
        assert_eq!(response.headers()["x-robots-tag"], "noindex, nofollow");

        let body = to_bytes(response.into_body(), usize::MAX).await.unwrap();
        assert_eq!(&body[..], b"<h1>hello</h1>");
    }

    #[tokio::test]
    async fn unknown_id_is_not_found() {
        let response = router(store())
            .oneshot(
                Request::builder()
                    .uri("/a/does-not-exist")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::NOT_FOUND);
    }
}
