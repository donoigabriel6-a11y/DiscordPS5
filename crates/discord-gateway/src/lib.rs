//! Discord Gateway Client
//!
//! Handles WebSocket connection to Discord's Gateway API,
//! including identification, heartbeating, session resumption,
//! and event dispatching.

pub mod connection;
pub mod events;
pub mod heartbeat;
pub mod resume;
pub mod dispatcher;

pub use connection::*;
pub use events::*;
pub use heartbeat::*;
pub use resume::*;
pub use dispatcher::*;