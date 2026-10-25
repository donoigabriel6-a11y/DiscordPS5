//! Discord REST API Client
//!
//! Handles HTTP requests to Discord's REST API with rate limiting,
//! authentication, and error handling.

pub mod client;
pub mod endpoints;
pub mod models;
pub mod auth;
pub mod rate_limit;

pub use client::*;
pub use endpoints::*;
pub use models::*;
pub use auth::*;
pub use rate_limit::*;