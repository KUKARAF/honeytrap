use super::client_ip;
use crate::render;
use crate::template::TemplateStore;
use axum::extract::{ConnectInfo, State};
use axum::http::{header, HeaderMap, StatusCode, Uri};
use axum::response::{IntoResponse, Response};
use std::net::SocketAddr;
use std::sync::Arc;

pub struct AppState {
    pub store: TemplateStore,
    pub salt: String,
    pub max_bytes: usize,
    pub trusted_proxy: bool,
}

pub async fn handle(
    State(state): State<Arc<AppState>>,
    ConnectInfo(peer): ConnectInfo<SocketAddr>,
    headers: HeaderMap,
    uri: Uri,
) -> Response {
    let client_ip = client_ip::resolve(&headers, peer, state.trusted_proxy);
    let host = headers
        .get(header::HOST)
        .and_then(|v| v.to_str().ok())
        .unwrap_or("")
        .to_lowercase();
    let path = uri.path();

    let Some(result) = render::render_for_path(&state.store, path, &client_ip, &host, &state.salt)
    else {
        tracing::info!(event = "miss", client_ip = %client_ip, host = %host, path = %path, status = 404);
        return StatusCode::NOT_FOUND.into_response();
    };

    let rendered = match result {
        Ok(r) => r,
        Err(e) => {
            tracing::error!(event = "render_error", client_ip = %client_ip, host = %host, path = %path, error = %e);
            return StatusCode::INTERNAL_SERVER_ERROR.into_response();
        }
    };

    debug_assert!(
        rendered.bytes.len() <= state.max_bytes,
        "startup validation should have guaranteed this"
    );

    let hash = blake3::hash(&rendered.bytes).to_hex().to_string();
    tracing::info!(
        event = "served",
        client_ip = %client_ip,
        host = %host,
        path = %path,
        template = %rendered.template,
        bytes = rendered.bytes.len(),
        response_hash = %hash,
    );

    (
        [
            (header::CONTENT_TYPE, "text/plain"),
            (header::CACHE_CONTROL, "no-store"),
        ],
        rendered.bytes,
    )
        .into_response()
}
