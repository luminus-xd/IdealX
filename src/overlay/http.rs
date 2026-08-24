use std::{sync::Arc, time::Duration};

use axum::{
    body::Body,
    extract::{
        ws::{CloseFrame, Message, WebSocket, WebSocketUpgrade},
        Path, State,
    },
    http::{header, HeaderValue, Request, StatusCode},
    middleware::{self, Next},
    response::{Html, IntoResponse, Response},
    routing::get,
    Router,
};
use futures_util::StreamExt;
use serde::Deserialize;
use tokio::sync::{broadcast, Semaphore};
use tokio_util::sync::CancellationToken;

use super::{hub::OverlayHub, model::OverlayEvent};

const OVERLAY_HTML: &str = include_str!("../../assets/overlay.html");
const OVERLAY_CSS: &str = include_str!("../../assets/overlay.css");
const OVERLAY_JS: &str = include_str!("../../assets/overlay.js");
const MAX_WS_CONNECTIONS: usize = 8;
const MAX_AUTH_FRAME_BYTES: usize = 8 * 1024;
const AUTH_TIMEOUT: Duration = Duration::from_secs(5);
const HEARTBEAT_INTERVAL: Duration = Duration::from_secs(10);

#[derive(Clone)]
struct HttpState {
    hub: Arc<OverlayHub>,
    connection_slots: Arc<Semaphore>,
}

#[derive(Deserialize)]
struct AuthMessage {
    #[serde(rename = "type")]
    kind: String,
    capability: String,
}

pub async fn serve(
    listener: tokio::net::TcpListener,
    hub: Arc<OverlayHub>,
    shutdown: CancellationToken,
) -> anyhow::Result<()> {
    let app = router(hub);

    axum::serve(listener, app)
        .with_graceful_shutdown(shutdown.cancelled_owned())
        .await?;
    Ok(())
}

fn router(hub: Arc<OverlayHub>) -> Router {
    let state = HttpState {
        hub,
        connection_slots: Arc::new(Semaphore::new(MAX_WS_CONNECTIONS)),
    };

    Router::new()
        .route("/healthz", get(healthz))
        .route("/overlay/{public_id}", get(overlay_page))
        .route("/assets/overlay.css", get(overlay_css))
        .route("/assets/overlay.js", get(overlay_js))
        .route("/ws/overlay/{public_id}", get(overlay_socket))
        .layer(middleware::from_fn(security_headers))
        .with_state(state)
}

async fn healthz() -> &'static str {
    "ok"
}

async fn overlay_page(State(state): State<HttpState>, Path(public_id): Path<String>) -> Response {
    if state.hub.has_public_id(&public_id).await {
        Html(OVERLAY_HTML).into_response()
    } else {
        StatusCode::NOT_FOUND.into_response()
    }
}

async fn overlay_css() -> Response {
    (
        [(header::CONTENT_TYPE, "text/css; charset=utf-8")],
        OVERLAY_CSS,
    )
        .into_response()
}

async fn overlay_js() -> Response {
    (
        [(header::CONTENT_TYPE, "text/javascript; charset=utf-8")],
        OVERLAY_JS,
    )
        .into_response()
}

async fn overlay_socket(
    ws: WebSocketUpgrade,
    State(state): State<HttpState>,
    Path(public_id): Path<String>,
) -> Response {
    if !state.hub.has_public_id(&public_id).await {
        return StatusCode::NOT_FOUND.into_response();
    }

    let Ok(connection_permit) = state.connection_slots.clone().try_acquire_owned() else {
        return StatusCode::SERVICE_UNAVAILABLE.into_response();
    };

    ws.max_message_size(MAX_AUTH_FRAME_BYTES)
        .max_frame_size(MAX_AUTH_FRAME_BYTES)
        .on_upgrade(move |socket| async move {
            let _connection_permit = connection_permit;
            handle_socket(socket, state.hub, public_id).await;
        })
}

async fn handle_socket(mut socket: WebSocket, hub: Arc<OverlayHub>, public_id: String) {
    let auth_frame = match tokio::time::timeout(AUTH_TIMEOUT, socket.next()).await {
        Ok(Some(Ok(Message::Text(text)))) if text.len() <= MAX_AUTH_FRAME_BYTES => text,
        _ => {
            close_socket(&mut socket, 1008, "authentication required").await;
            return;
        }
    };

    let Ok(auth) = serde_json::from_str::<AuthMessage>(&auth_frame) else {
        close_socket(&mut socket, 1008, "invalid authentication").await;
        return;
    };
    if auth.kind != "auth" {
        close_socket(&mut socket, 1008, "invalid authentication").await;
        return;
    }

    let Some((snapshot, mut events)) = hub.authenticate(&public_id, &auth.capability).await else {
        close_socket(&mut socket, 1008, "authentication failed").await;
        return;
    };
    drop(auth);

    if send_json(&mut socket, &OverlayEvent::Snapshot { snapshot })
        .await
        .is_err()
    {
        return;
    }

    let mut heartbeat = tokio::time::interval(HEARTBEAT_INTERVAL);
    heartbeat.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Skip);

    loop {
        tokio::select! {
            event = events.recv() => {
                match event {
                    Ok(event) => {
                        let terminal = matches!(event, OverlayEvent::SessionEnded { .. });
                        if send_json(&mut socket, &event).await.is_err() {
                            return;
                        }
                        if terminal {
                            close_socket(&mut socket, 1000, "session ended").await;
                            return;
                        }
                    }
                    Err(broadcast::error::RecvError::Lagged(_)) => {
                        close_socket(&mut socket, 1011, "state resync required").await;
                        return;
                    }
                    Err(broadcast::error::RecvError::Closed) => return,
                }
            }
            _ = heartbeat.tick() => {
                if socket
                    .send(Message::Text(r#"{"type":"heartbeat"}"#.into()))
                    .await
                    .is_err()
                {
                    return;
                }
            }
            incoming = socket.next() => {
                match incoming {
                    Some(Ok(Message::Close(_))) | None | Some(Err(_)) => return,
                    Some(Ok(Message::Ping(payload))) => {
                        if socket.send(Message::Pong(payload)).await.is_err() {
                            return;
                        }
                    }
                    Some(Ok(_)) => {}
                }
            }
        }
    }
}

async fn send_json<T: serde::Serialize>(socket: &mut WebSocket, value: &T) -> Result<(), ()> {
    let json = serde_json::to_string(value).map_err(|_| ())?;
    socket
        .send(Message::Text(json.into()))
        .await
        .map_err(|_| ())
}

async fn close_socket(socket: &mut WebSocket, code: u16, reason: &'static str) {
    let _ = socket
        .send(Message::Close(Some(CloseFrame {
            code,
            reason: reason.into(),
        })))
        .await;
}

async fn security_headers(request: Request<Body>, next: Next) -> Response {
    let mut response = next.run(request).await;
    let headers = response.headers_mut();
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
        "x-robots-tag",
        HeaderValue::from_static("noindex, nofollow"),
    );
    headers.insert(
        header::CONTENT_SECURITY_POLICY,
        HeaderValue::from_static(
            "default-src 'none'; script-src 'self'; style-src 'self'; img-src https://cdn.discordapp.com; connect-src 'self' ws: wss:; base-uri 'none'; form-action 'none'; frame-ancestors 'none'",
        ),
    );
    response
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::overlay::model::{OverlayTheme, StartSession};
    use axum::body::to_bytes;
    use tower::ServiceExt;

    fn test_hub() -> Arc<OverlayHub> {
        Arc::new(OverlayHub::new(
            Duration::from_secs(20),
            Duration::from_secs(60),
        ))
    }

    #[tokio::test]
    async fn healthcheck_has_security_headers() {
        let response = router(test_hub())
            .oneshot(
                Request::builder()
                    .uri("/healthz")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::OK);
        assert_eq!(
            response.headers()[header::CACHE_CONTROL],
            "no-store, private"
        );
        assert_eq!(response.headers()[header::REFERRER_POLICY], "no-referrer");
        assert_eq!(response.headers()["x-robots-tag"], "noindex, nofollow");
        assert!(response.headers()[header::CONTENT_SECURITY_POLICY]
            .to_str()
            .unwrap()
            .contains("default-src 'none'"));
    }

    #[tokio::test]
    async fn viewer_html_contains_neither_capability_nor_comments() {
        let hub = test_hub();
        let credentials = hub
            .start(StartSession {
                guild_id: 1,
                channel_id: 2,
                owner_user_id: 3,
                theme: OverlayTheme::Dark,
                show_avatar: true,
                display_duration: Duration::from_secs(20),
            })
            .await
            .unwrap();
        let response = router(hub)
            .oneshot(
                Request::builder()
                    .uri(format!("/overlay/{}", credentials.public_id))
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::OK);
        let body = to_bytes(response.into_body(), usize::MAX).await.unwrap();
        let html = String::from_utf8(body.to_vec()).unwrap();
        assert!(!html.contains(&credentials.capability));
        assert!(!html.contains("discord_message_id"));
        assert!(html.contains("/assets/overlay.js"));
    }

    #[tokio::test]
    async fn unknown_viewer_id_is_not_served() {
        let response = router(test_hub())
            .oneshot(
                Request::builder()
                    .uri("/overlay/not-a-session")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::NOT_FOUND);
    }
}
