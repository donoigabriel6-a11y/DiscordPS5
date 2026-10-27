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

/// Initialize the PS5 platform
/// This initializes threading and other platform subsystems.
pub fn init_platform() -> crate::Result<()> {
    info!("Initializing PS5 platform abstraction layer");

    // Initialize threading subsystem
    threading::init_threading()?;

    // Initialize filesystem
    filesystem::init_data_dir()?;

    info!("PS5 platform initialized successfully");
    Ok(())
}