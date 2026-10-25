//! Discord PS5 Client - Main Entry Point
//!
//! A Discord client for jailbroken PS5 with voice calling and screen sharing support.

use std::sync::Arc;
use std::time::Duration;
use anyhow::{Context, Result};
use clap::Parser;
use parking_lot::RwLock;
use tokio::sync::mpsc;
use tracing::{error, info, warn, Level};
use tracing_subscriber::{fmt, EnvFilter};

use discord_common::{config::Config, discord_types::*};
use discord_gateway::GatewayClient;
use discord_rest::RestClient;
use discord_voice::{VoiceConfig, VoiceConnection};
use discord_ui::{App, Renderer};
use ps5_platform::{graphics, input, audio, init_platform};

mod state;
mod events;
mod voice;
mod screen_share;

use state::AppState;
use events::EventHandler;
use voice::VoiceManager;
use screen_share::ScreenShareManager;

/// Command line arguments
#[derive(Parser, Debug)]
#[command(name = "discord-ps5", version, about = "Discord client for PS5")]
struct Args {
    /// Configuration file path
    #[arg(short, long, default_value = "config.toml")]
    config: String,

    /// Log level
    #[arg(short, long, default_value = "info")]
    log_level: String,

    /// Run in headless mode (no UI)
    #[arg(long)]
    headless: bool,

    /// Enable debug logging
    #[arg(short, long)]
    debug: bool,
}

#[tokio::main]
async fn main() -> Result<()> {
    // Parse arguments
    let args = Args::parse();

    // Initialize logging
    init_logging(&args.log_level, args.debug)?;

    info!("Starting Discord PS5 Client v0.1.0");

    // Initialize PS5 platform
    init_platform().context("Failed to initialize PS5 platform")?;
    info!("PS5 platform initialized");

    // Load configuration
    let config = Config::load(&args.config).await
        .context("Failed to load configuration")?;
    info!("Configuration loaded");

    // Create application state
    let app_state = Arc::new(RwLock::new(AppState::new(config.clone())));

    // Initialize graphics (DRM/KMS/GBM/EGL)
    let (surface, egl_context) = graphics::init_graphics(&config.graphics)
        .context("Failed to initialize graphics")?;
    info!("Graphics initialized: {}x{}", surface.width(), surface.height());

    // Initialize audio
    let audio_system = audio::init_audio(&config.audio)
        .context("Failed to initialize audio")?;
    info!("Audio system initialized");

    // Initialize input (DualSense)
    let dualsense = input::DualSense::new()
        .context("Failed to initialize DualSense controller")?;
    info!("DualSense controller initialized");

    // Create REST client
    let rest = RestClient::new(config.rest.clone())
        .context("Failed to create REST client")?;

    // Create Gateway client
    let (gateway, gateway_events) = GatewayClient::new(config.gateway.clone())
        .context("Failed to create Gateway client")?;

    // Create voice configuration
    let voice_config = VoiceConfig::from(&config.voice);

    // Create UI renderer
    let mut renderer = Renderer::new(surface, egl_context)
        .context("Failed to create UI renderer")?;

    // Create UI application
    let mut app = App::new()
        .context("Failed to create UI application")?;

    // Initialize app with Discord clients
    app.initialize(gateway, rest, voice_config).await
        .context("Failed to initialize application")?;

    // Create event handler
    let mut event_handler = EventHandler::new(app_state.clone());

    // Create voice manager
    let mut voice_manager = VoiceManager::new(app_state.clone());

    // Create screen share manager
    let mut screen_share = ScreenShareManager::new(app_state.clone());

    // Main loop
    info!("Entering main loop");
    let mut running = true;
    let mut last_frame = std::time::Instant::now();

    while running {
        let now = std::time::Instant::now();
        let delta_time = now.duration_since(last_frame).as_secs_f32();
        last_frame = now;

        // Cap frame time
        let delta_time = delta_time.min(1.0 / 30.0);

        // Process platform events
        process_platform_events(&mut app, &dualsense);

        // Process gateway events
        event_handler.process_gateway_events(&mut app).await;

        // Process voice events
        voice_manager.process_voice_events(&mut app).await;

        // Process screen share events
        screen_share.process_events(&mut app).await;

        // Update and render UI
        if !args.headless {
            running = app.update(&mut renderer, delta_time)?;
        }

        // Check for shutdown
        if matches!(app.state(), discord_ui::AppState::Shutdown) {
            running = false;
        }

        // Yield to other tasks
        tokio::task::yield_now().await;
    }

    // Shutdown
    info!("Shutting down...");
    app.shutdown().await?;
    renderer.shutdown();

    info!("Discord PS5 Client shut down cleanly");
    Ok(())
}

/// Initialize logging
fn init_logging(log_level: &str, debug: bool) -> Result<()> {
    let level = if debug {
        Level::DEBUG
    } else {
        log_level.parse().unwrap_or(Level::INFO)
    };

    let filter = EnvFilter::try_from_default_env()
        .unwrap_or_else(|_| EnvFilter::new(format!("discord_ps5={},discord_gateway={},discord_rest={},discord_voice={}",
            level, level, level, level)));

    fmt::Subscriber::builder()
        .with_env_filter(filter)
        .with_target(false)
        .with_thread_ids(true)
        .with_file(true)
        .with_line_number(true)
        .init();

    Ok(())
}

/// Process platform events (input, etc.)
fn process_platform_events(app: &mut App, dualsense: &input::DualSense) {
    // Poll DualSense events
    // In a real implementation, this would be done in the App's handle_input
    // For now, we just ensure the controller is polled
    let _ = dualsense.poll_event();
}