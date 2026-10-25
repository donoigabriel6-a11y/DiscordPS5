//! Voice Manager
//!
//! Manages voice channel connections, audio I/O, and voice state.

use std::sync::Arc;
use anyhow::{Context, Result};
use parking_lot::RwLock;
use tokio::sync::mpsc;
use tracing::{debug, error, info, warn};
use cpal::{Stream, StreamConfig, SampleRate, SampleFormat};
use discord_common::discord_types::*;
use discord_voice::{VoiceConnection, VoiceConfig, VoiceEvent, VoiceConnectionState};
use crate::state::AppState;

/// Voice manager for handling voice connections
pub struct VoiceManager {
    app_state: Arc<AppState>,
    audio_input_stream: Option<Stream>,
    audio_output_stream: Option<Stream>,
    input_config: Option<StreamConfig>,
    output_config: Option<StreamConfig>,
}

impl VoiceManager {
    pub fn new(app_state: Arc<AppState>) -> Self {
        Self {
            app_state,
            audio_input_stream: None,
            audio_output_stream: None,
            input_config: None,
            output_config: None,
        }
    }

    /// Initialize audio devices
    pub async fn initialize_audio(&mut self) -> Result<()> {
        use cpal::traits::{DeviceTrait, HostTrait};

        let host = cpal::default_host();

        // Get input device (microphone)
        let input_device = host.default_input_device()
            .context("No input device available")?;

        // Get output device (speakers/headset)
        let output_device = host.default_output_device()
            .context("No output device available")?;

        info!("Audio input: {}", input_device.name()?);
        info!("Audio output: {}", output_device.name()?);

        // Configure input stream
        let input_config = input_device.default_input_config()
            .context("Failed to get default input config")?;

        // Configure output stream
        let output_config = output_device.default_output_config()
            .context("Failed to get default output config")?;

        self.input_config = Some(input_config.config());
        self.output_config = Some(output_config.config());

        Ok(())
    }

    /// Join a voice channel
    pub async fn join_channel(&self, channel_id: ChannelId) -> Result<()> {
        info!("Joining voice channel: {}", channel_id);

        *self.app_state.voice_channel_id.write() = Some(channel_id);
        *self.app_state.voice_state.write() = VoiceConnectionState::Connecting;

        // Get voice connection
        if let Some(voice) = self.app_state.voice_connection() {
            voice.connect().await?;
        }

        // Start audio streams
        self.start_audio_streams().await?;

        Ok(())
    }

    /// Leave voice channel
    pub async fn leave_channel(&self) -> Result<()> {
        info!("Leaving voice channel");

        // Stop audio streams
        self.stop_audio_streams();

        // Disconnect voice
        if let Some(voice) = self.app_state.voice_connection() {
            voice.disconnect().await?;
        }

        *self.app_state.voice_channel_id.write() = None;
        *self.app_state.voice_state.write() = VoiceConnectionState::Disconnected;

        Ok(())
    }

    /// Start audio input/output streams
    async fn start_audio_streams(&mut self) -> Result<()> {
        use cpal::traits::{DeviceTrait, HostTrait};
        use cpal::{Sample, FromSample};

        let host = cpal::default_host();
        let input_device = host.default_input_device()
            .context("No input device")?;
        let output_device = host.default_output_device()
            .context("No output device")?;

        let input_config = self.input_config.as_ref()
            .context("Input config not initialized")?;
        let output_config = self.output_config.as_ref()
            .context("Output config not initialized")?;

        // Create voice connection reference
        let voice_conn = self.app_state.voice_connection();

        // Input stream (microphone -> Discord)
        let input_stream = input_device.build_input_stream(
            &input_config.config(),
            move |data: &[f32], _: &cpal::InputCallbackInfo| {
                // Convert f32 to i16 and send to voice connection
                if let Some(voice) = &voice_conn {
                    let pcm: Vec<i16> = data.iter()
                        .map(|s| (s.clamp(-1.0, 1.0) * i16::MAX as f32) as i16)
                        .collect();

                    // Send audio to voice connection
                    tokio::spawn(async move {
                        let _ = voice.write_audio(&pcm).await;
                    });
                }
            },
            |err| error!("Input stream error: {}", err),
            None,
        ).context("Failed to build input stream")?;

        // Output stream (Discord -> speakers)
        let output_stream = output_device.build_output_stream(
            &output_config.config(),
            move |data: &mut [f32], _: &cpal::OutputCallbackInfo| {
                // In a real implementation, we'd pull from the voice receive pipeline
                // For now, output silence
                for sample in data.iter_mut() {
                    *sample = 0.0;
                }
            },
            |err| error!("Output stream error: {}", err),
            None,
        ).context("Failed to build output stream")?;

        input_stream.play().context("Failed to start input stream")?;
        output_stream.play().context("Failed to start output stream")?;

        self.audio_input_stream = Some(input_stream);
        self.audio_output_stream = Some(output_stream);

        info!("Audio streams started");
        Ok(())
    }

    /// Stop audio streams
    fn stop_audio_streams(&mut self) {
        if let Some(stream) = self.audio_input_stream.take() {
            drop(stream);
        }
        if let Some(stream) = self.audio_output_stream.take() {
            drop(stream);
        }
        info!("Audio streams stopped");
    }

    /// Toggle mute
    pub async fn toggle_mute(&self) -> Result<()> {
        // In a real implementation, this would mute the local audio input
        info!("Toggle mute");
        Ok(())
    }

    /// Toggle deafen
    pub async fn toggle_deafen(&self) -> Result<()> {
        // In a real implementation, this would mute the local audio output
        info!("Toggle deafen");
        Ok(())
    }

    /// Set input volume (0-200)
    pub fn set_input_volume(&self, volume: u8) {
        // Would adjust input gain
        debug!("Input volume: {}%", volume);
    }

    /// Set output volume (0-200)
    pub fn set_output_volume(&self, volume: u8) {
        // Would adjust output gain
        debug!("Output volume: {}%", volume);
    }

    /// Process voice events
    pub async fn process_voice_events(&mut self, app: &mut crate::App) {
        // Voice events are processed in App's update loop
        // This is kept for future extensibility
    }

    /// Get current voice state
    pub fn voice_state(&self) -> VoiceConnectionState {
        *self.app_state.voice_state.read()
    }

    /// Check if in voice channel
    pub fn in_voice_channel(&self) -> bool {
        self.app_state.voice_channel_id.read().is_some()
    }

    /// Get current voice channel ID
    pub fn current_channel(&self) -> Option<ChannelId> {
        *self.app_state.voice_channel_id.read()
    }
}