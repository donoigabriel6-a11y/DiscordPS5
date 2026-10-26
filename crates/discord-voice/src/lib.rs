//! Discord Voice Client
//!
//! Handles WebRTC voice connections to Discord voice channels,
//! including signaling, ICE negotiation, DTLS/SRTP, and Opus audio.

pub mod connection;
pub mod signaling;
pub mod audio;
pub mod opus;
pub mod ice;

// Re-export common types
pub use discord_common::{DiscordError, Result};

// Voice-specific configuration
#[derive(Debug, Clone)]
pub struct VoiceConfig {
    pub endpoint: String,
    pub guild_id: String,
    pub user_id: String,
    pub session_id: String,
    pub token: String,
    pub ice_servers: Vec<webrtc::ice_transport::ice_server::RTCIceServer>,
}

impl VoiceConfig {
    pub fn new(
        endpoint: String,
        guild_id: String,
        user_id: String,
        session_id: String,
        token: String,
    ) -> Self {
        Self {
            endpoint,
            guild_id,
            user_id,
            session_id,
            token,
            ice_servers: crate::ice::default_ice_servers(),
        }
    }
}

// Voice events
#[derive(Debug, Clone)]
pub enum VoiceEvent {
    Connected,
    Disconnected,
    AudioReceived { ssrc: u32, data: Vec<u8> },
    SpeakingUpdate { ssrc: u32, speaking: bool },
    Error(String),
}

// Voice connection state
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum VoiceConnectionState {
    Disconnected,
    Connecting,
    Connected,
    Reconnecting,
    Failed,
}

pub use connection::*;
pub use signaling::*;
pub use audio::*;
pub use opus::*;
pub use ice::*;