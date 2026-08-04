pub mod client_ip;
pub mod handlers;

use axum::routing::any;
use axum::Router;
use handlers::AppState;
use std::net::SocketAddr;
use std::sync::Arc;
use tower_http::catch_panic::CatchPanicLayer;

pub fn build_router(state: Arc<AppState>) -> Router {
    Router::new()
        .fallback(any(handlers::handle))
        .layer(CatchPanicLayer::new())
        .with_state(state)
}

pub async fn run_server(listen: SocketAddr, state: Arc<AppState>) -> anyhow::Result<()> {
    let app = build_router(state).into_make_service_with_connect_info::<SocketAddr>();
    let listener = tokio::net::TcpListener::bind(listen).await?;
    tracing::info!(event = "listening", addr = %listen);
    axum::serve(listener, app).await?;
    Ok(())
}
