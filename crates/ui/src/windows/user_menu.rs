//! User Menu Window (Bottom Left)
//!
//! Displays the current user's profile and status menu.

use imgui::{Ui, Window, MouseButton};
use crate::theme::*;
use discord_common::discord_types::*;

/// User menu window (popout from bottom left)
pub struct UserMenuWindow {
    /// Status options
    statuses: Vec<(UserStatus, &'static str, [f32; 4])>,
}

impl UserMenuWindow {
    pub fn new() -> Self {
        Self {
            statuses: vec![
                (UserStatus::Online, "Online", DISCORD_GREEN),
                (UserStatus::Idle, "Idle", DISCORD_YELLOW),
                (UserStatus::DoNotDisturb, "Do Not Disturb", DISCORD_RED),
                (UserStatus::Invisible, "Invisible", DISCORD_TEXT_MUTED),
            ],
        }
    }

    pub fn render(
        &mut self,
        ui: &Ui,
        open: &mut bool,
        current_user: &Option<User>
    ) {
        let viewport = ui.main_viewport();
        let pos = [10.0, viewport.work_size[1] - 200.0];

        Window::new("User Menu")
            .flags(imgui::WindowFlags::NO_TITLE_BAR
                | imgui::WindowFlags::NO_RESIZE
                | imgui::WindowFlags::NO_MOVE
                | imgui::WindowFlags::ALWAYS_AUTO_RESIZE
                | imgui::WindowFlags::POPUP)
            .position(pos, imgui::Condition::Always)
            .opened(open)
            .build(ui, || {
                self.render_content(ui, current_user);
            });
    }

    fn render_content(&self, ui: &Ui, current_user: &Option<User>) {
        if let Some(user) = current_user {
            // Avatar
            ui.image(
                imgui::TextureId::from(0), // Placeholder
                [40.0, 40.0]
            );

            ui.same_line();

            // Username and status
            ui.text(&user.username);
            ui.text_colored(DISCORD_TEXT_MUTED, format!("#{}", user.discriminator.unwrap_or(0)));

            ui.separator();

            // Status picker
            ui.text("Set Status");
            for (status, label, color) in &self.statuses {
                let mut selected = false; // Would check current status
                if ui.selectable_config(label)
                    .flags(imgui::SelectableFlags::DONT_CLOSE_POPUPS)
                    .build() {
                    // TODO: Set status
                }
                // Status indicator dot
                ui.same_line();
                let cursor = ui.cursor_screen_pos();
                ui.get_window_draw_list().add_circle_filled(
                    [cursor[0] - 10.0, cursor[1] + 8.0],
                    5.0,
                    (*color).into()
                ).build();
            }

            ui.separator();

            // Custom status
            let mut custom_status = String::new();
            ui.input_text("##CustomStatus", &mut custom_status)
                .hint("Set a custom status...")
                .build();

            ui.separator();

            // Menu items
            if ui.menu_item("Profile") {
                // TODO
            }
            if ui.menu_item("Settings") {
                // TODO: Open settings
            }
            ui.separator();
            if ui.menu_item("Log Out") {
                // TODO
            }
        } else {
            ui.text_colored(DISCORD_TEXT_MUTED, "Not logged in");
        }
    }
}

impl Default for UserMenuWindow {
    fn default() -> Self {
        Self::new()
    }
}