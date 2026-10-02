// Application configuration
use serde::Deserialize;
use std::env;

#[derive(Debug, Deserialize)]
pub struct Config {
    pub database: DatabaseConfig,
    pub server: ServerConfig,
    pub stellar: StellarConfig,
    pub rate_limits: RateLimitConfig,
}

#[derive(Debug, Clone, Copy, Deserialize)]
pub struct RateLimitRule {
    pub requests_per_minute: u32,
    pub burst: u32,
}

#[derive(Debug, Clone, Copy, Deserialize)]
pub struct RateLimitConfig {
    pub public: RateLimitRule,
    pub authenticated: RateLimitRule,
    pub payment: RateLimitRule,
    pub merchant: RateLimitRule,
    pub auth: RateLimitRule,
    pub health: RateLimitRule,
}

impl RateLimitConfig {
    pub fn from_env() -> Self {
        Self {
            public: rule_from_env("PUBLIC", 100),
            authenticated: rule_from_env("AUTHENTICATED", 1000),
            payment: rule_from_env("PAYMENT", 10),
            merchant: rule_from_env("MERCHANT", 100),
            auth: rule_from_env("AUTH", 100),
            health: rule_from_env("HEALTH", 1_000_000),
        }
    }
}

fn rule_from_env(name: &str, default_requests_per_minute: u32) -> RateLimitRule {
    let requests_per_minute = env::var(format!("RATE_LIMIT_{name}_PER_MINUTE"))
        .ok()
        .and_then(|value| value.parse::<u32>().ok())
        .filter(|value| *value > 0)
        .unwrap_or(default_requests_per_minute);
    let burst = env::var(format!("RATE_LIMIT_{name}_BURST"))
        .ok()
        .and_then(|value| value.parse::<u32>().ok())
        .filter(|value| *value > 0)
        .unwrap_or(requests_per_minute);

    RateLimitRule {
        requests_per_minute,
        burst,
    }
}

#[derive(Debug, Deserialize)]
pub struct DatabaseConfig {
    pub url: String,
    pub max_connections: u32,
}

#[derive(Debug, Deserialize)]
pub struct ServerConfig {
    pub host: String,
    pub port: u16,
}

#[derive(Debug, Deserialize)]
pub struct StellarConfig {
    pub network: String,
    pub horizon_url: String,
    pub network_passphrase: String,
}

impl Config {
    pub fn from_env() -> Result<Self, env::VarError> {
        Ok(Config {
            database: DatabaseConfig {
                url: env::var("DATABASE_URL")?,
                max_connections: env::var("DATABASE_MAX_CONNECTIONS")
                    .ok()
                    .and_then(|v| v.parse().ok())
                    .unwrap_or(5),
            },
            server: ServerConfig {
                host: env::var("HOST").unwrap_or_else(|_| "0.0.0.0".to_string()),
                port: env::var("PORT")
                    .ok()
                    .and_then(|v| v.parse().ok())
                    .unwrap_or(3001),
            },
            stellar: StellarConfig {
                network: env::var("STELLAR_NETWORK").unwrap_or_else(|_| "testnet".to_string()),
                horizon_url: env::var("STELLAR_HORIZON_URL")
                    .unwrap_or_else(|_| "https://horizon-testnet.stellar.org".to_string()),
                network_passphrase: env::var("STELLAR_NETWORK_PASSPHRASE")
                    .unwrap_or_else(|_| "Test SDF Network ; September 2015".to_string()),
            },
            rate_limits: RateLimitConfig::from_env(),
        })
    }
}
