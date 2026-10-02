use axum::{
    extract::Request,
    extract::{ConnectInfo, State},
    http::{header, HeaderValue, StatusCode},
    middleware::Next,
    response::{IntoResponse, Response},
    Json,
};
use crate::config::{RateLimitConfig, RateLimitRule};
use serde_json::json;
use std::{
    collections::HashMap,
    net::SocketAddr,
    sync::{Arc, Mutex},
    time::{Duration, Instant, SystemTime, UNIX_EPOCH},
};

#[derive(Clone)]
pub struct RateLimitState {
    config: RateLimitConfig,
    buckets: Arc<Mutex<HashMap<String, Bucket>>>,
}

#[derive(Clone, Debug)]
pub struct RateLimitIdentity(pub String);

struct Bucket {
    tokens: f64,
    updated_at: Instant,
    last_seen: Instant,
}

struct Decision {
    allowed: bool,
    remaining: u32,
    reset_at: u64,
    retry_after: u64,
}

impl RateLimitState {
    pub fn new(config: RateLimitConfig) -> Self {
        Self {
            config,
            buckets: Arc::new(Mutex::new(HashMap::new())),
        }
    }

    fn check(&self, key: String, rule: RateLimitRule) -> Decision {
        let now = Instant::now();
        let capacity = f64::from(rule.burst);
        let refill_per_second = f64::from(rule.requests_per_minute) / 60.0;
        let mut buckets = self.buckets.lock().unwrap_or_else(|error| error.into_inner());

        if buckets.len() > 10_000 {
            buckets.retain(|_, bucket| now.duration_since(bucket.last_seen) < Duration::from_secs(600));
        }

        let bucket = buckets.entry(key).or_insert(Bucket {
            tokens: capacity,
            updated_at: now,
            last_seen: now,
        });
        bucket.tokens = (bucket.tokens
            + now.duration_since(bucket.updated_at).as_secs_f64() * refill_per_second)
            .min(capacity);
        bucket.updated_at = now;
        bucket.last_seen = now;

        let allowed = bucket.tokens >= 1.0;
        if allowed {
            bucket.tokens -= 1.0;
        }

        let until_full = Duration::from_secs_f64((capacity - bucket.tokens) / refill_per_second);
        let retry_after = if allowed {
            0
        } else {
            (1.0 / refill_per_second).ceil() as u64
        };

        Decision {
            allowed,
            remaining: bucket.tokens.floor() as u32,
            reset_at: unix_timestamp_after(until_full),
            retry_after,
        }
    }
}

pub async fn rate_limit_middleware(
    State(state): State<RateLimitState>,
    req: Request,
    next: Next,
) -> Response {
    let path = req.uri().path();
    let authenticated_identity = req
        .extensions()
        .get::<RateLimitIdentity>()
        .map(|identity| identity.0.clone());
    let identity = authenticated_identity
        .as_ref()
        .map(|identity| format!("user:{identity}"))
        .or_else(|| {
            req.extensions()
                .get::<ConnectInfo<SocketAddr>>()
                .map(|ConnectInfo(address)| format!("ip:{}", address.ip()))
        })
        .unwrap_or_else(|| "ip:unknown".to_string());
    let (group, rule) = select_rule(&state.config, path, authenticated_identity.is_some());
    let decision = state.check(format!("{group}:{identity}"), rule);

    let mut response = if decision.allowed {
        next.run(req).await
    } else {
        tracing::warn!(%identity, %path, limit = rule.requests_per_minute, "rate limit exceeded");
        (
            StatusCode::TOO_MANY_REQUESTS,
            [(header::RETRY_AFTER, decision.retry_after.to_string())],
            Json(json!({"error": "Too many requests", "status": 429})),
        )
            .into_response()
    };

    response.headers_mut().insert(
        "x-ratelimit-limit",
        HeaderValue::from(rule.requests_per_minute),
    );
    response.headers_mut().insert(
        "x-ratelimit-remaining",
        HeaderValue::from(decision.remaining),
    );
    response.headers_mut().insert(
        "x-ratelimit-reset",
        HeaderValue::from_str(&decision.reset_at.to_string()).expect("valid timestamp header"),
    );

    if !decision.allowed {
        response.headers_mut().insert(
            header::RETRY_AFTER,
            HeaderValue::from(decision.retry_after),
        );
    }

    response
}

fn select_rule(config: &RateLimitConfig, path: &str, authenticated: bool) -> (&'static str, RateLimitRule) {
    let path = path.strip_prefix("/api").unwrap_or(path);
    if path == "/health" {
        ("health", config.health)
    } else if path.starts_with("/payments") || path.starts_with("/payment-requests") {
        ("payment", config.payment)
    } else if path.starts_with("/auth") {
        ("auth", config.auth)
    } else if path == "/merchants/me" || (authenticated && !path.starts_with("/merchants")) {
        ("authenticated", config.authenticated)
    } else if path.starts_with("/merchants") {
        ("merchant", config.merchant)
    } else {
        ("public", config.public)
    }
}

fn unix_timestamp_after(duration: Duration) -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .saturating_add(duration)
        .as_secs()
}

#[cfg(test)]
#[path = "../middleware_tests.rs"]
mod tests;
