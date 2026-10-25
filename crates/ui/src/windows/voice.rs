//! Voice Window (Voice Controls Popout)
//!
//! Displays voice channel connection status and controls.

use imgui::{Ui, TableFlags};
use crate::theme::*;
use crate::components::VoiceControls;
use discord_common::discord_types::*;
use discord_voice::{VoiceConnection, VoiceConnectionState, VoiceEvent};
use std::sync::Arc;
use parking_lot::RwLock;

/// Voice window (popout for detailed voice controls)
pub struct VoiceWindow {
    /// Show connected users
    show_users: bool,
    /// Scroll position
    scroll_y: f32,
}

impl VoiceWindow {
    pub fn new() -> Self {
        Self {
            show_users: true,
            scroll_y: 0.0,
        }
    }

    pub fn render(
        &mut self,
        ui: &Ui,
        open: &mut bool,
        voice_state: &VoiceConnectionState,
        voice: Option<&VoiceConnection>
    ) {
        Window::new("Voice Connected")
            .flags(imgui::WindowFlags::NO_COLLAPSE)
            .size([320.0, 400.0], imgui::Condition::FirstUseEver)
            .position([100.0, 100.0], imgui::Condition::FirstUseEver)
            .opened(open)
            .build(ui, || {
                self.render_content(ui, voice_state, voice);
            });
    }

    fn render_content(
        &mut self,
        ui: &Ui,
        voice_state: &VoiceConnectionState,
        voice: Option<&VoiceConnection>
    ) {
        // Connection status
        let (status_text, status_color) = match voice_state {
            VoiceConnectionState::Disconnected => ("Disconnected", DISCORD_TEXT_MUTED),
            VoiceConnectionState::Connecting => ("Connecting...", DISCORD_YELLOW),
            VoiceConnectionState::Connected => ("Connected", DISCORD_GREEN),
            VoiceConnectionState::Reconnecting => ("Reconnecting...", DISCORD_YELLOW),
            VoiceConnectionState::Failed => ("Failed", DISCORD_RED),
        };

        ui.text_colored(status_color, format!("Status: {}", status_text));
        ui.separator();

        // Voice controls
        VoiceControls::render_static(ui, voice_state, voice);

        ui.separator();

        // Connected users
        if self.show_users {
            if ui.collapsing_header("Connected Users", TreeNodeFlags::DEFAULT_OPEN) {
                // In a real implementation, this would show actual users from voice state
                ui.text_colored(DISCORD_TEXT_MUTED, "No users connected");
            }
        }

        // Voice settings
        if ui.collapsing_header("Voice Settings", TreeNodeFlags::DEFAULT_OPEN) {
            // Input volume
            let mut input_vol = 100;
            ui.slider_config("Input Volume", 0, 200, &mut input_vol)
                .format("%d%%")
                .build();

            // Output volume
            let mut output_vol = 100;
            ui.slider_config("Output Volume", 0, 200, &mut output_vol)
                .format("%d%%")
                .build();

            // Input mode
            ui.text("Input Mode:");
            ui.radio_button("Voice Activity", &mut 0, 0);
            ui.radio_button("Push to Talk", &mut 0, 1);

            // Noise suppression
            let mut noise_suppression = true;
            ui.checkbox("Noise Suppression", &mut noise_suppression);

            // Echo cancellation
            let mut echo_cancellation = true;
            ui.checkbox("Echo Cancellation", &mut echo_cancellation);

            // Automatic gain control
            let mut agc = true;
            ui.checkbox("Automatic Gain Control", &mut agc);
        }

        // Advanced
        if ui.collapsing_header("Advanced", TreeNodeFlags::DEFAULT_OPEN) {
            ui.text_colored(DISCORD_TEXT_MUTED, "Region: Automatic");
            ui.text_colored(DISCORD_TEXT_MUTED, "Bitrate: 64 kbps");
            ui.text_colored(DISCORD_TEXT_MUTED, "Codec: Opus");

            if ui.button("Debug Logs") {
                // TODO: Show debug logs
            }

            if ui.button("RTC Connection Info") {
                // TODO: Show WebRTC connection details
            }
        }
    }
}

// Need to import Window
use imgui::Window;

impl Default for VoiceWindow {
    fn default() -> Self {
        Self::new()
    }
}