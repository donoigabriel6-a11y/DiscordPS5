//! Servers Window (Left Sidebar)
//!
//! Displays the list of servers/guilds with icons.

use imgui::{Ui, Image, TreeNodeFlags, SelectableFlags, MouseButton};
use crate::theme::*;
use discord_common::discord_types::*;
use std::collections::HashMap;

/// Server/Guild list window
pub struct ServersWindow {
    /// Scroll position
    scroll_y: f32,
    /// Hovered server index
    hovered_index: Option<usize>,
    /// Expanded folders
    expanded_folders: Vec<GuildFolderId>,
}

impl ServersWindow {
    pub fn new() -> Self {
        Self {
            scroll_y: 0.0,
            hovered_index: None,
            expanded_folders: Vec::new(),
        }
    }

    pub fn render(
        &mut self,
        ui: &Ui,
        guilds: &HashMap<GuildId, Guild>,
        selected: &SelectedItem
    ) {
        let draw_list = ui.get_window_draw_list();
        let cursor_pos = ui.cursor_screen_pos();
        let content_region = ui.content_region_avail();

        // Background
        draw_list.add_rect_filled(
            cursor_pos,
            [cursor_pos[0] + content_region[0], cursor_pos[1] + content_region[1]],
            DISCORD_SIDEBAR_BG.into(),
        ).build();

        // Add "Add Server" and "Discover" buttons at top
        let button_size = 48.0;
        let spacing = 8.0;
        let start_y = cursor_pos[1] + spacing;

        // Add Server button (+)
        self.render_circle_button(
            ui,
            draw_list,
            "+",
            [cursor_pos[0] + (content_region[0] - button_size) * 0.5, start_y],
            button_size,
            DISCORD_ACCENT,
            DISCORD_ACCENT_HOVER,
            matches!(selected, SelectedItem::None),
        );

        // Discover button (compass)
        self.render_circle_button(
            ui,
            draw_list,
            "🔍",
            [cursor_pos[0] + (content_region[0] - button_size) * 0.5, start_y + button_size + spacing],
            button_size,
            DISCORD_SIDEBAR_BG,
            DISCORD_HOVER,
            false,
        );

        // Separator
        let separator_y = start_y + button_size * 2.0 + spacing * 2.0;
        draw_list.add_line(
            [cursor_pos[0] + 8.0, separator_y],
            [cursor_pos[0] + content_region[0] - 8.0, separator_y],
            DISCORD_BORDER.into(),
            1.0,
        ).build();

        // Server list
        let list_start_y = separator_y + 8.0;
        let available_height = content_region[1] - (list_start_y - cursor_pos[1]) - 8.0;

        ui.set_cursor_pos([8.0, list_start_y - cursor_pos[1]]);

        // Collect guilds into a vector for consistent ordering
        let mut guild_list: Vec<_> = guilds.values().collect();
        guild_list.sort_by(|a, b| {
            // Sort by name, but keep DMs at top if any
            a.name.cmp(&b.name)
        });

        for (index, guild) in guild_list.iter().enumerate() {
            let button_y = list_start_y + index as f32 * (button_size + 4.0);

            // Skip if out of view
            if button_y > cursor_pos[1] + content_region[1] {
                break;
            }

            let is_selected = matches!(selected, SelectedItem::Server(id) if *id == guild.id);
            let is_hovered = self.hovered_index == Some(index);

            // Server icon background
            let icon_pos = [cursor_pos[0] + (content_region[0] - button_size) * 0.5, button_y];
            let icon_rect_min = icon_pos;
            let icon_rect_max = [icon_pos[0] + button_size, icon_pos[1] + button_size];

            // Background
            let bg_color = if is_selected {
                DISCORD_ACCENT
            } else if is_hovered {
                DISCORD_HOVER
            } else {
                [0.0, 0.0, 0.0, 0.0] // Transparent
            };

            if bg_color[3] > 0.0 {
                draw_list.add_rect_filled(
                    icon_rect_min,
                    icon_rect_max,
                    bg_color.into(),
                    4.0,
                ).build();
            }

            // Server icon/initial
            let icon_text = guild.icon
                .as_ref()
                .map(|_| "📷") // Would render actual icon
                .unwrap_or_else(|| &guild.name[0..1].to_uppercase());

            let text_color = if is_selected || is_hovered {
                DISCORD_TEXT
            } else {
                DISCORD_TEXT_MUTED
            };

            draw_list.add_text(
                [icon_pos[0] + button_size * 0.5 - 8.0, icon_pos[1] + button_size * 0.5 - 8.0],
                text_color.into(),
                icon_text,
            ).build();

            // Unread indicator (placeholder)
            if false { // guild.has_unread
                let dot_pos = [icon_rect_max[0] - 8.0, icon_pos[1] + 4.0];
                draw_list.add_circle_filled(dot_pos, 4.0, DISCORD_GREEN.into()).build();
            }

            // Handle click
            if ui.is_mouse_hovering_rect(icon_rect_min, icon_rect_max) {
                self.hovered_index = Some(index);
                if ui.is_mouse_clicked(MouseButton::Left) {
                    // Selection handled by parent
                }
            }
        }
    }

    /// Render a circular button
    fn render_circle_button(
        &self,
        ui: &Ui,
        draw_list: &imgui::DrawListMut,
        text: &str,
        pos: [f32; 2],
        size: f32,
        bg_color: [f32; 4],
        hover_color: [f32; 4],
        selected: bool,
    ) {
        let center = [pos[0] + size * 0.5, pos[1] + size * 0.5];
        let radius = size * 0.5 - 2.0;

        let is_hovered = ui.is_mouse_hovering_rect(pos, [pos[0] + size, pos[1] + size]);
        let color = if selected {
            bg_color
        } else if is_hovered {
            hover_color
        } else {
            bg_color
        };

        draw_list.add_circle_filled(center, radius, color.into()).build();

        // Text
        let text_size = ui.calc_text_size(text);
        draw_list.add_text(
            [center[0] - text_size[0] * 0.5, center[1] - text_size[1] * 0.5],
            DISCORD_TEXT.into(),
            text,
        ).build();
    }
}

impl Default for ServersWindow {
    fn default() -> Self {
        Self::new()
    }
}