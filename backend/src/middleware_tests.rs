use super::{rate_limit_middleware, RateLimitIdentity, RateLimitState};
use crate::config::{RateLimitConfig, RateLimitRule};
use axum::{
    body::Body,
    http::{Request, StatusCode},
    middleware::from_fn_with_state,
    routing::get,
    Router,
};
use std::net::{IpAddr, Ipv4Addr, SocketAddr};
use tower::ServiceExt;

fn test_rule(requests_per_minute: u32, burst: u32) -> RateLimitRule {
    RateLimitRule {
        requests_per_minute,
        burst,
    }
}

fn test_app() -> Router {
    let state = RateLimitState::new(RateLimitConfig {
        public: test_rule(60, 2),
        authenticated: test_rule(600, 2),
        payment: test_rule(60, 1),
        merchant: test_rule(60, 2),
        auth: test_rule(60, 2),
        health: test_rule(60, 10),
    });

    Router::new()
        .route("/api/payments", get(|| async { StatusCode::OK }))
        .route("/api/merchants", get(|| async { StatusCode::OK }))
        .route("/api/merchants/me", get(|| async { StatusCode::OK }))
        .route("/api/health", get(|| async { StatusCode::OK }))
        .layer(from_fn_with_state(state, rate_limit_middleware))
}

fn request(path: &str, user: Option<&str>) -> Request<Body> {
    let mut request = Request::builder().uri(path).body(Body::empty()).unwrap();
    request.extensions_mut().insert(axum::extract::ConnectInfo(
        SocketAddr::new(IpAddr::V4(Ipv4Addr::LOCALHOST), 1234),
    ));
    if let Some(user) = user {
        request
            .extensions_mut()
            .insert(RateLimitIdentity(user.to_string()));
    }
    request
}

#[tokio::test]
async fn payment_limit_returns_429_and_rate_limit_headers() {
    let app = test_app();
    let first = app.clone().oneshot(request("/api/payments", None)).await.unwrap();
    assert_eq!(first.status(), StatusCode::OK);
    assert_eq!(first.headers()["x-ratelimit-limit"], "60");
    assert_eq!(first.headers()["x-ratelimit-remaining"], "0");
    assert!(first.headers().contains_key("x-ratelimit-reset"));

    let second = app.oneshot(request("/api/payments", None)).await.unwrap();
    assert_eq!(second.status(), StatusCode::TOO_MANY_REQUESTS);
    assert_eq!(second.headers()["retry-after"], "1");
    assert_eq!(second.headers()["x-ratelimit-limit"], "60");
}

#[tokio::test]
async fn authenticated_users_have_independent_buckets() {
    let app = test_app();
    let first = app
        .clone()
        .oneshot(request("/api/payments", Some("merchant-a")))
        .await
        .unwrap();
    let second = app
        .oneshot(request("/api/payments", Some("merchant-b")))
        .await
        .unwrap();

    assert_eq!(first.status(), StatusCode::OK);
    assert_eq!(second.status(), StatusCode::OK);
}

#[tokio::test]
async fn authenticated_merchant_route_uses_authenticated_limit() {
    let response = test_app()
        .oneshot(request("/api/merchants/me", Some("merchant-a")))
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);
    assert_eq!(response.headers()["x-ratelimit-limit"], "600");
}

#[tokio::test]
async fn health_endpoint_uses_its_separate_bucket() {
    let app = test_app();
    for _ in 0..2 {
        let response = app
            .clone()
            .oneshot(request("/api/health", None))
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::OK);
    }
}