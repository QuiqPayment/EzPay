// API route definitions
// This module will contain all API route handlers

pub mod merchants;
pub mod payments;
pub mod health;

pub use merchants::merchant_routes;
pub use payments::payment_routes;
pub use health::{health_routes, health_routes_with_db};
