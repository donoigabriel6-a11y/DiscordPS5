//! Discord Voice Client
//!
//! Handles WebRTC voice connections to Discord voice channels,
//! including signaling, ICE negotiation, DTLS/SRTP, and Opus audio.

pub mod connection;
pub mod signaling;
pub mod audio;
pub mod opus;
pub mod ice;

pub use connection::*;
pub use signaling::*;
pub use audio::*;
pub use opus::*;
pub use ice::*;