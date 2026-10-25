//! Error types for the Discord PS5 client

use thiserror::Error;

#[derive(Error, Debug)]
pub enum DiscordError {
    #[error("Network error: {0}")]
    Network(#[from] std::io::Error),

    #[error("Serialization error: {0}")]
    Serialization(#[from] serde_json::Error),

    #[error("WebSocket error: {0}")]
    WebSocket(#[from] tungstenite::Error),

    #[error("HTTP error: {0}")]
    Http(#[from] reqwest::Error),

    #[error("Database error: {0}")]
    Database(#[from] rusqlite::Error),

    #[error("Audio error: {0}")]
    Audio(String),

    #[error("Graphics error: {0}")]
    Graphics(String),

    #[error("Input error: {0}")]
    Input(String),

    #[error("Authentication error: {0}")]
    Auth(String),

    #[error("Rate limited: retry after {retry_after} seconds")]
    RateLimited { retry_after: u64 },

    #[error("Invalid token")]
    InvalidToken,

    #[error("Gateway error: {code} - {message}")]
    Gateway { code: u16, message: String },

    #[error("Voice connection error: {0}")]
    VoiceConnection(String),

    #[error("Configuration error: {0}")]
    Config(String),

    #[error("Platform error: {0}")]
    Platform(String),

    #[error("Not implemented: {0}")]
    NotImplemented(String),

    #[error("Unknown error: {0}")]
    Unknown(String),
}

pub type Result<T> = std::result::Result<T, DiscordError>;