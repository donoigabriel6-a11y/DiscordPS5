//! Settings Window
//!
//! Main settings window with tabs for different categories.

use imgui::{Ui, TableFlags, TabBarFlags, TabItemFlags};
use crate::theme::*;
use crate::components::*;
use discord_common::discord_types::*;
use std::sync::Arc;
use parking_lot::RwLock;

/// Settings window
pub struct SettingsWindow {
    /// Current tab
    current_tab: SettingsTab,
}

impl SettingsWindow {
    pub fn new() -> Self {
        Self {
            current_tab: SettingsTab::Account,
        }
    }

    pub fn render(
        &mut self,
        ui: &Ui,
        open: &mut bool,
        current_tab: &mut SettingsTab,
        current_user: &Option<User>
    ) {
        self.current_tab = *current_tab;

        Window::new("User Settings")
            .flags(imgui::WindowFlags::NO_COLLAPSE)
            .size([700.0, 550.0], imgui::Condition::FirstUseEver)
            .position_center(imgui::Condition::FirstUseEver)
            .opened(open)
            .build(ui, || {
                self.render_tabs(ui, current_tab, current_user);
            });
    }

    fn render_tabs(&mut self, ui: &Ui, current_tab: &mut SettingsTab, current_user: &Option<User>) {
        if ui.begin_tab_bar("##SettingsTabs", TabBarFlags::REORDERABLE) {
            // Account tab
            if ui.begin_tab_item("My Account") {
                self.render_account_tab(ui, current_user);
                ui.end_tab_item();
            }

            // Voice tab
            if ui.begin_tab_item("Voice & Video") {
                self.render_voice_tab(ui);
                ui.end_tab_item();
            }

            // Video tab
            if ui.begin_tab_item("Video") {
                self.render_video_tab(ui);
                ui.end_tab_item();
            }

            // Notifications tab
            if ui.begin_tab_item("Notifications") {
                self.render_notifications_tab(ui);
                ui.end_tab_item();
            }

            // Keybinds tab
            if ui.begin_tab_item("Keybinds") {
                self.render_keybinds_tab(ui);
                ui.end_tab_item();
            }

            // Appearance tab
            if ui.begin_tab_item("Appearance") {
                self.render_appearance_tab(ui);
                ui.end_tab_item();
            }

            // Advanced tab
            if ui.begin_tab_item("Advanced") {
                self.render_advanced_tab(ui);
                ui.end_tab_item();
            }

            ui.end_tab_bar();
        }

        *current_tab = self.current_tab;
    }

    fn render_account_tab(&mut self, ui: &Ui, current_user: &Option<User>) {
        ui.spacing();

        if let Some(user) = current_user {
            // Avatar
            ui.image(
                imgui::TextureId::from(0), // Placeholder
                [128.0, 128.0]
            );

            ui.same_line();
            ui.dummy([20.0, 0.0]);
            ui.same_line();

            // Username
            ui.text(format!("{}#{}", user.username, user.discriminator.unwrap_or(0)));
            ui.text_colored(DISCORD_TEXT_MUTED, format!("User ID: {}", user.id));

            // Email
            if let Some(email) = &user.email {
                ui.text_colored(DISCORD_TEXT_MUTED, format!("Email: {}", email));
            }

            ui.separator();
            ui.spacing();

            // Buttons
            if ui.button("Edit Profile", [120.0, 0.0]) {
                // TODO
            }
            ui.same_line();
            if ui.button("Change Password", [120.0, 0.0]) {
                // TODO
            }
            ui.same_line();
            if ui.button("Enable 2FA", [120.0, 0.0]) {
                // TODO
            }
        }
    }

    fn render_voice_tab(&mut self, ui: &Ui) {
        ui.spacing();

        // Input device
        ui.text("Input Device:");
        ui.combo("##InputDevice", &mut 0, &["Default", "DualSense Mic", "USB Headset"]);

        ui.spacing();

        // Output device
        ui.text("Output Device:");
        ui.combo("##OutputDevice", &mut 0, &["Default", "TV Speakers", "Headset", "Controller Speaker"]);

        ui.separator();
        ui.spacing();

        // Volume sliders
        let mut input_vol = 100;
        ui.slider_config("Input Volume", 0, 200, &mut input_vol)
            .format("%d%%")
            .build();

        let mut output_vol = 100;
        ui.slider_config("Output Volume", 0, 200, &mut output_vol)
            .format("%d%%")
            .build();

        ui.separator();
        ui.spacing();

        // Input mode
        ui.text("Input Mode:");
        let mut input_mode = 0;
        ui.radio_button("Voice Activity", &mut input_mode, 0);
        ui.radio_button("Push to Talk", &mut input_mode, 1);

        if input_mode == 1 {
            ui.indent();
            ui.text("Shortcut: ");
            ui.same_line();
            ui.button("Set Keybind", [100.0, 0.0]);
            ui.unindent();
        }

        ui.separator();
        ui.spacing();

        // Voice processing
        let mut noise_suppression = true;
        ui.checkbox("Noise Suppression", &mut noise_suppression);

        let mut echo_cancellation = true;
        ui.checkbox("Echo Cancellation", &mut echo_cancellation);

        let mut agc = true;
        ui.checkbox("Automatic Gain Control", &mut agc);

        let mut vad = true;
        ui.checkbox("Voice Activity Detection", &mut vad);

        ui.separator();
        ui.spacing();

        // Attenuation
        ui.text("Attenuation:");
        let mut attenuation = 50;
        ui.slider_config("When others speak", 0, 100, &mut attenuation)
            .format("%d%%")
            .build();

        let mut attenuation_self = 0;
        ui.slider_config("When I speak", 0, 100, &mut attenuation_self)
            .format("%d%%")
            .build();
    }

    fn render_video_tab(&mut self, ui: &Ui) {
        ui.spacing();

        ui.text("Camera:");
        ui.combo("##Camera", &mut 0, &["None", "PS5 Camera", "USB Camera"]);

        ui.spacing();

        let mut enable_video = false;
        ui.checkbox("Enable Video", &mut enable_video);

        if enable_video {
            ui.indent();
            ui.text("Resolution:");
            ui.combo("##Resolution", &mut 0, &["720p", "1080p", "480p"]);

            ui.text("Frame Rate:");
            ui.combo("##Framerate", &mut 0, &["30 FPS", "60 FPS"]);

            let mut hardware_accel = true;
            ui.checkbox("Hardware Acceleration", &mut hardware_accel);
            ui.unindent();
        }

        ui.separator();
        ui.spacing();

        // Screen share
        ui.text("Screen Share:");
        let mut screen_share_fps = 30;
        ui.slider_config("Max Frame Rate", 5, 60, &mut screen_share_fps)
            .format("%d FPS")
            .build();

        let mut screen_share_quality = 2;
        ui.combo("Quality", &mut screen_share_quality, &["Low", "Medium", "High", "Lossless"]);
    }

    fn render_notifications_tab(&mut self, ui: &Ui) {
        ui.spacing();

        let mut enable_notifications = true;
        ui.checkbox("Enable Desktop Notifications", &mut enable_notifications);

        ui.separator();
        ui.spacing();

        // Notification settings
        let settings = [
            ("All Messages", true),
            ("Only @mentions", false),
            ("Nothing", false),
        ];

        ui.text("Default Notification Setting:");
        for (label, _) in &settings {
            ui.radio_button(label, &mut 0, 0);
        }

        ui.separator();
        ui.spacing();

        // Sound settings
        ui.text("Sounds:");
        let mut sounds = [
            ("Message", true),
            ("Mention", true),
            ("Join/Leave", true),
            ("Typing", false),
        ];

        for (label, enabled) in &mut sounds {
            ui.checkbox(label, enabled);
        }
    }

    fn render_keybinds_tab(&mut self, ui: &Ui) {
        ui.spacing();

        ui.text_colored(DISCORD_TEXT_MUTED, "Keybinds are configured per-platform.");
        ui.text_colored(DISCORD_TEXT_MUTED, "On PS5, use the DualSense controller mapping.");

        ui.separator();
        ui.spacing();

        // Keybind list
        let keybinds = [
            ("Push to Talk", "Unbound"),
            ("Toggle Mute", "Unbound"),
            ("Toggle Deafen", "Unbound"),
            ("Overlay", "Unbound"),
            ("Quick Switcher", "Unbound"),
        ];

        for (action, binding) in &keybinds {
            ui.text(action);
            ui.same_line();
            ui.text_colored(DISCORD_TEXT_MUTED, binding);
            ui.same_line();
            if ui.small_button(&format!("Edit##{}", action)) {
                // TODO
            }
        }
    }

    fn render_appearance_tab(&mut self, ui: &Ui) {
        ui.spacing();

        // Theme
        ui.text("Theme:");
        let mut theme = 0;
        ui.radio_button("Dark", &mut theme, 0);
        ui.same_line();
        ui.radio_button("Light", &mut theme, 1);
        ui.same_line();
        ui.radio_button("Sync with System", &mut theme, 2);

        ui.separator();
        ui.spacing();

        // Message display
        ui.text("Message Display:");
        let mut compact = false;
        ui.checkbox("Compact Mode", &mut compact);

        let mut show_avatars = true;
        ui.checkbox("Show Avatars", &mut show_avatars);

        let mut show_timestamps = true;
        ui.checkbox("Show Timestamps", &mut show_timestamps);

        ui.separator();
        ui.spacing();

        // Font scaling
        ui.text("Font Scaling:");
        let mut scale = 100;
        ui.slider_config("##FontScale", 50, 150, &mut scale)
            .format("%d%%")
            .build();

        // Zoom level
        ui.text("Zoom Level:");
        let mut zoom = 100;
        ui.slider_config("##Zoom", 50, 200, &mut zoom)
            .format("%d%%")
            .build();
    }

    fn render_advanced_tab(&mut self, ui: &Ui) {
        ui.spacing();

        ui.text_colored(DISCORD_ACCENT, "⚠️ Advanced Settings");
        ui.text_colored(DISCORD_TEXT_MUTED, "Be careful when changing these settings.");

        ui.separator();
        ui.spacing();

        // Developer mode
        let mut dev_mode = false;
        ui.checkbox("Developer Mode", &mut dev_mode);
        ui.text_colored(DISCORD_TEXT_MUTED, "Enables developer tools and copy ID options");

        ui.separator();
        ui.spacing();

        // Hardware acceleration
        let mut hw_accel = true;
        ui.checkbox("Hardware Acceleration", &mut hw_accel);

        // Log level
        ui.text("Log Level:");
        ui.combo("##LogLevel", &mut 0, &["Error", "Warn", "Info", "Debug", "Trace"]);

        ui.separator();
        ui.spacing();

        // Cache
        if ui.button("Clear Cache", [120.0, 0.0]) {
            // TODO
        }
        ui.same_line();
        if ui.button("Reset Settings", [120.0, 0.0]) {
            // TODO
        }

        ui.separator();
        ui.spacing();

        // Version info
        ui.text_colored(DISCORD_TEXT_MUTED, "Discord PS5 Client v0.1.0");
        ui.text_colored(DISCORD_TEXT_MUTED, "Built with Rust + ImGui");
    }
}

impl Default for SettingsWindow {
    fn default() -> Self {
        Self::new()
    }
}