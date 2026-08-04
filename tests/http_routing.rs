use axum::body::Body;
use axum::extract::connect_info::ConnectInfo;
use axum::http::{Request, StatusCode};
use honeytrap::http::build_router;
use honeytrap::http::handlers::AppState;
use honeytrap::template::TemplateStore;
use http_body_util::BodyExt;
use std::net::SocketAddr;
use std::path::Path;
use std::sync::Arc;
use tower::ServiceExt;

fn state(trusted_proxy: bool, fallback: bool) -> Arc<AppState> {
    let store = TemplateStore::load(Path::new("templates")).expect("load shipped templates");
    Arc::new(AppState {
        store,
        salt: "test-salt".to_string(),
        max_bytes: 8192,
        trusted_proxy,
        fallback,
    })
}

fn request(path: &str, host: &str, xff: Option<&str>) -> Request<Body> {
    let mut builder = Request::builder().uri(path).header("Host", host);
    if let Some(xff) = xff {
        builder = builder.header("x-forwarded-for", xff);
    }
    let mut req = builder.body(Body::empty()).unwrap();
    let peer: SocketAddr = "9.9.9.9:1234".parse().unwrap();
    req.extensions_mut().insert(ConnectInfo(peer));
    req
}

#[tokio::test]
async fn unknown_path_404s_when_fallback_disabled() {
    let app = build_router(state(false, false));
    let response = app
        .oneshot(request("/definitely-not-a-real-path", "example.com", None))
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::NOT_FOUND);
}

#[tokio::test]
async fn known_path_200s_with_expected_headers() {
    let app = build_router(state(false, false));
    let response = app
        .oneshot(request("/.env", "example.com", None))
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::OK);
    assert_eq!(
        response.headers().get("content-type").unwrap(),
        "text/plain"
    );
    assert_eq!(response.headers().get("cache-control").unwrap(), "no-store");
    let body = response.into_body().collect().await.unwrap().to_bytes();
    assert!(!body.is_empty());
    assert!(body.len() <= 8192);
}

#[tokio::test]
async fn trusted_proxy_off_ignores_xff() {
    let app_a = build_router(state(false, false));
    let app_b = build_router(state(false, false));
    let r1 = app_a
        .oneshot(request("/.env", "example.com", Some("1.1.1.1")))
        .await
        .unwrap();
    let r2 = app_b
        .oneshot(request("/.env", "example.com", Some("2.2.2.2")))
        .await
        .unwrap();
    let b1 = r1.into_body().collect().await.unwrap().to_bytes();
    let b2 = r2.into_body().collect().await.unwrap().to_bytes();
    assert_eq!(b1, b2, "XFF must be ignored when --trusted-proxy is off");
}

#[tokio::test]
async fn trusted_proxy_on_honors_xff() {
    let app_a = build_router(state(true, false));
    let app_b = build_router(state(true, false));
    let r1 = app_a
        .oneshot(request("/.env", "example.com", Some("1.1.1.1")))
        .await
        .unwrap();
    let r2 = app_b
        .oneshot(request("/.env", "example.com", Some("2.2.2.2")))
        .await
        .unwrap();
    let b1 = r1.into_body().collect().await.unwrap().to_bytes();
    let b2 = r2.into_body().collect().await.unwrap().to_bytes();
    assert_ne!(
        b1, b2,
        "different XFF values must diverge when --trusted-proxy is on"
    );
}

#[tokio::test]
async fn unmatched_path_serves_fallback_when_enabled() {
    let app = build_router(state(false, true));
    let response = app
        .oneshot(request("/.env.bak", "example.com", None))
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::OK);
    assert_eq!(
        response.headers().get("content-type").unwrap(),
        "text/plain"
    );
    let body = response.into_body().collect().await.unwrap().to_bytes();
    assert!(!body.is_empty(), "fallback response must have content");
}

#[tokio::test]
async fn same_unmatched_path_is_stable() {
    let app_a = build_router(state(false, true));
    let app_b = build_router(state(false, true));
    let r1 = app_a
        .oneshot(request("/.env.bak", "example.com", None))
        .await
        .unwrap();
    let r2 = app_b
        .oneshot(request("/.env.bak", "example.com", None))
        .await
        .unwrap();
    let b1 = r1.into_body().collect().await.unwrap().to_bytes();
    let b2 = r2.into_body().collect().await.unwrap().to_bytes();
    assert_eq!(
        b1, b2,
        "the same unmatched path must return identical fallback content"
    );
}

#[tokio::test]
async fn different_unmatched_paths_diverge() {
    let app_a = build_router(state(false, true));
    let app_b = build_router(state(false, true));
    let r1 = app_a
        .oneshot(request("/.env.bak", "example.com", None))
        .await
        .unwrap();
    let r2 = app_b
        .oneshot(request("/backup.tar.gz", "example.com", None))
        .await
        .unwrap();
    let b1 = r1.into_body().collect().await.unwrap().to_bytes();
    let b2 = r2.into_body().collect().await.unwrap().to_bytes();
    assert_ne!(
        b1, b2,
        "different unmatched paths must not render byte-identical output"
    );
}
