//! Voice Controls Component
//!
//! Displays voice connection controls (mute, deafen, disconnect, etc.)

use imgui::{Ui, Window};
use crate::theme::*;
use discord_voice::{VoiceConnection, VoiceConnectionState};

/// Voice controls component (shown at bottom of channel list)
pub struct VoiceControls {
    /// Show expanded controls
    expanded: bool,
}

impl VoiceControls {
    pub fn new() -> Self {
        Self {
            expanded: false,
        }
    }

    pub fn render(
        &mut self,
        ui: &Ui,
        voice_state: &VoiceConnectionState,
        voice_channel_id: &Option<ChannelId>,
        voice: Option<&VoiceConnection>
    ) {
        let is_connected = voice_channel_id.is_some();
        let is_connecting = matches!(voice_state, VoiceConnectionState::Connecting | VoiceConnectionState::Reconnecting);

        if !is_connected && !is_connecting {
            // No voice connection - show "Join Voice" area
            self.render_join_area(ui);
            return;
        }

        // Connected state - show controls
        self.render_connected_controls(ui, voice_state, voice);
    }

    fn render_join_area(&self, ui: &Ui) {
        let avail = ui.content_region_avail();
        let btn_width = (avail[0] - 16.0) * 0.5;

        // Voice connected indicator (grayed out)
        ui.push_style_color(imgui::StyleColor::Button, DISCORD_SIDEBAR_BG.into());
        ui.push_style_color(imgui::StyleColor::ButtonHovered, DISCORD_HOVER.into());
        ui.push_style_color(imgui::StyleColor::ButtonActive, DISCORD_ACTIVE.into());
        ui.push_style_color(imgui::StyleColor::Text, DISCORD_TEXT_MUTED.into());

        if ui.button("No Voice Channel", [avail[0], 60.0]) {
            // Would open channel list to join
        }

        ui.pop_style_color(4);
    }

    fn render_connected_controls(
        &mut self,
        ui: &Ui,
        voice_state: &VoiceConnectionState,
        voice: Option<&VoiceConnection>
    ) {
        let is_connected = matches!(voice_state, VoiceConnectionState::Connected);
        let is_connecting = matches!(voice_state, VoiceConnectionState::Connecting | VoiceConnectionState::Reconnecting);

        let avail = ui.content_region_avail();

        // User info area
        ui.push_style_color(imgui::StyleColor::ChildBg, DISCORD_SIDEBAR_BG.into());
        ui.push_style_var(imgui::StyleVar::ChildRounding, 4.0);

        ui.child_window("##VoiceUserInfo")
            .flags(imgui::WindowFlags::NO_SCROLLBAR)
            .size([avail[0], 60.0])
            .build(|| {
                // Avatar
                ui.image(imgui::TextureId::from(0), [40.0, 40.0]);
                ui.same_line();

                // Username and status
                ui.text("You");
                ui.same_line();

                // Status indicator
                let (status_text, status_color) = match voice_state {
                    VoiceConnectionState::Connected => ("Connected", DISCORD_GREEN),
                    VoiceConnectionState::Connecting => ("Connecting...", DISCORD_YELLOW),
                    VoiceConnectionState::Reconnecting => ("Reconnecting...", DISCORD_YELLOW),
                    VoiceConnectionState::Disconnected => ("Disconnected", DISCORD_TEXT_MUTED),
                    VoiceConnectionState::Failed => ("Failed", DISCORD_RED),
                };

                let cursor = ui.cursor_screen_pos();
                ui.get_window_draw_list().add_circle_filled(
                    [cursor[0] + 6.0, cursor[1] + 6.0],
                    6.0,
                    status_color.into()
                ).build();

                ui.dummy([16.0, 0.0]);
                ui.same_line();
                ui.text_colored(status_color, status_text);
            });

        ui.pop_style_var();
        ui.pop_style_color();

        ui.spacing();

        // Control buttons
        let btn_size = 40.0;
        let spacing = 8.0;
        let total_width = btn_size * 4.0 + spacing * 3.0;
        let start_x = (avail[0] - total_width) * 0.5;

        ui.set_cursor_pos_x(start_x);

        // Mute button
        self.render_control_button(
            ui,
            "🎤",
            "Mute (M)",
            false, // muted state
            DISCORD_RED,
            btn_size
        );
        ui.same_line();
        ui.dummy([spacing, 0.0]);
        ui.same_line();

        // Deafen button
        self.render_control_button(
            ui,
            "🔇",
            "Deafen (D)",
            false, // deafened state
            DISCORD_RED,
            btn_size
        );
        ui.same_line();
        ui.dummy([spacing, 0.0]);
        ui.same_line();

        // Settings button
        self.render_control_button(
            ui,
            "⚙",
            "Voice Settings",
            false,
            DISCORD_SIDEBAR_BG,
            btn_size
        );
        ui.same_line();
        ui.dummy([spacing, 0.0]);
        ui.same_line();

        // Disconnect button
        let disconnect_color = if is_connected { DISCORD_RED } else { DISCORD_SIDEBAR_BG };
        self.render_control_button(
            ui,
            "✕",
            "Disconnect",
            false,
            disconnect_color,
            btn_size
        );

        // Expandable area for more controls
        if self.expanded && is_connected {
            ui.separator();
            ui.spacing();

            // Volume sliders
            ui.text("Input Volume");
            let mut input_vol = 100;
            ui.slider_config("##InputVol", 0, 200, &mut input_vol)
                .format("%d%%")
                .build();

            ui.text("Output Volume");
            let mut output_vol = 100;
            ui.slider_config("##OutputVol", 0, 200, &mut output_vol)
                .format("%d%%")
                .build();

            // Voice activity indicator
            ui.text("Voice Activity");
            ui.progress_bar(0.0, [avail[0] - 20.0, 8.0], "");
        }
    }

    fn render_control_button(
        &self,
        ui: &Ui,
        icon: &str,
        tooltip: &str,
        active: bool,
        active_color: [f32; 4],
        size: f32
    ) {
        let bg_color = if active { active_color } else { DISCORD_SIDEBAR_BG };
        let hover_color = if active {
            [active_color[0] * 0.8, active_color[1] * 0.8, active_color[2] * 0.8, active_color[3]]
        } else {
            DISCORD_HOVER
        };

        ui.push_style_color(imgui::StyleColor::Button, bg_color.into());
        ui.push_style_color(imgui::StyleColor::ButtonHovered, hover_color.into());
        ui.push_style_color(imgui::StyleColor::ButtonActive, DISCORD_ACTIVE.into());
        ui.push_style_color(imgui::StyleColor::Text, if active { [1.0, 1.0, 1.0, 1.0] } else { DISCORD_TEXT });
        ui.push_style_var(imgui::StyleVar::FrameRounding, size * 0.5);

        let clicked = ui.button(icon, [size, size]);

        ui.pop_style_var();
        ui.pop_style_color(4);

        if ui.is_item_hovered() {
            ui.set_tooltip(tooltip);
        }
    }

    /// Static render function for use in voice window
    pub fn render_static(
        ui: &Ui,
        voice_state: &VoiceConnectionState,
        voice: Option<&VoiceConnection>
    ) {
        let is_connected = matches!(voice_state, VoiceConnectionState::Connected);

        // Mute/Deafen buttons
        let btn_size = [100.0, 40.0];

        ui.push_style_var(imgui::StyleVar::FrameRounding, 4.0);

        // Mute
        let mut muted = false;
        ui.push_style_color(imgui::StyleColor::Button, if muted { DISCORD_RED } else { DISCORD_SIDEBAR_BG }.into());
        ui.push_style_color(imgui::StyleColor::ButtonHovered, if muted { DISCORD_RED } else { DISCORD_HOVER }.into());
        ui.push_style_color(imgui::StyleColor::Text, if muted { [1.0, 1.0, 1.0, 1.0] } else { DISCORD_TEXT }.into());
        if ui.button("🎤 Mute", btn_size) {
            // Toggle mute
        }
        ui.pop_style_color(3);

        ui.same_line();

        // Deafen
        let mut deafened = false;
        ui.push_style_color(imgui::StyleColor::Button, if deafened { DISCORD_RED } else { DISCORD_SIDEBAR_BG }.into());
        ui.push_style_color(imgui::StyleColor::ButtonHovered, if deafened { DISCORD_RED } else { DISCORD_HOVER }.into());
        ui.push_style_color(imgui::StyleColor::Text, if deafened { [1.0, 1.0, 1.0, 1.0] } else { DISCORD_TEXT }.into());
        if ui.button("🔇 Deafen", btn_size) {
            // Toggle deafen
        }
        ui.pop_style_color(3);

        ui.same_line();

        // Disconnect
        ui.push_style_color(imgui::StyleColor::Button, DISCORD_RED.into());
        ui.push_style_color(imgui::StyleColor::ButtonHovered, [0.8, 0.1, 0.1, 1.0].into());
        ui.push_style_color(imgui::StyleColor::Text, [1.0, 1.0, 1.0, 1.0].into());
        if ui.button("Disconnect", btn_size) {
            // Disconnect
        }
        ui.pop_style_color(3);

        ui.pop_style_var();
    }
}

impl Default for VoiceControls {
    fn default() -> Self {
        Self::new()
    }
}