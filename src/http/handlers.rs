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
    /// When true, an unmatched path is served a deterministically-picked
    /// fallback template instead of a 404.
    pub fallback: bool,
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

    let Some(result) = render::render_for_path(
        &state.store,
        path,
        &client_ip,
        &host,
        &state.salt,
        state.fallback,
    ) else {
        tracing::info!(event = "miss", client_ip = %client_ip, host = %host, path = %path, status = 404);
        return StatusCode::NOT_FOUND.into_response();
    };

    let mut rendered = match result {
        Ok(r) => r,
        Err(e) => {
            tracing::error!(event = "render_error", client_ip = %client_ip, host = %host, path = %path, error = %e);
            return StatusCode::INTERNAL_SERVER_ERROR.into_response();
        }
    };

    // Startup validation is the primary guarantee that templates fit the cap,
    // but enforce it at request time too: variable-length generators mean a
    // template's size can drift slightly with the seed, and startup only
    // samples a few seeds. Truncate on a char boundary so we never emit more
    // than the cap, and log it so an offending template is visible.
    if rendered.bytes.len() > state.max_bytes {
        tracing::warn!(
            event = "cap_exceeded",
            template = %rendered.template,
            bytes = rendered.bytes.len(),
            max_bytes = state.max_bytes,
        );
        // Back up off any UTF-8 continuation byte (0b10xxxxxx) so we never
        // split a multi-byte character. `cut < len` holds here because we only
        // enter this block when len > max_bytes.
        let mut cut = state.max_bytes;
        while cut > 0 && rendered.bytes.get(cut).is_some_and(|b| (b & 0xC0) == 0x80) {
            cut -= 1;
        }
        rendered.bytes.truncate(cut);
    }

    let hash = blake3::hash(&rendered.bytes).to_hex().to_string();
    tracing::info!(
        event = "served",
        client_ip = %client_ip,
        host = %host,
        path = %path,
        template = %rendered.template,
        fallback = rendered.fallback,
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
