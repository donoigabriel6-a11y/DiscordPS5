//! PS5 Platform Abstraction Layer
//!
//! Provides cross-platform abstractions for PS5-specific functionality:
//! - Graphics (DRM/KMS/GBM/EGL/OpenGL)
//! - Input (DualSense controller)
//! - Audio (capture/playback)
//! - Filesystem
//! - Network

pub mod graphics;
pub mod input;
pub mod audio;
pub mod filesystem;
pub mod network;
pub mod threading;

pub use graphics::*;
pub use input::*;
pub use audio::*;
pub use filesystem::*;
pub use network::*;
pub use threading::*;