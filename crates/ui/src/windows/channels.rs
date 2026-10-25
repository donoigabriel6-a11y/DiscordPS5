//! Channels Window (Second Sidebar)
//!
//! Displays the list of channels for the selected server.

use imgui::{Ui, TreeNodeFlags, SelectableFlags, MouseButton, TableFlags};
use crate::theme::*;
use discord_common::discord_types::*;
use std::collections::HashMap;

/// Channel list window
pub struct ChannelsWindow {
    /// Expanded categories
    expanded_categories: HashMap<ChannelId, bool>,
    /// Search filter
    search_text: String,
    /// Scroll position
    scroll_y: f32,
}

impl ChannelsWindow {
    pub fn new() -> Self {
        Self {
            expanded_categories: HashMap::new(),
            search_text: String::new(),
            scroll_y: 0.0,
        }
    }

    pub fn render(
        &mut self,
        ui: &Ui,
        channels: &HashMap<ChannelId, Channel>,
        guilds: &HashMap<GuildId, Guild>,
        selected: &SelectedItem
    ) {
        // Get selected guild
        let guild_id = match selected {
            SelectedItem::Server(id) => *id,
            _ => return,
        };

        let guild = guilds.get(&guild_id);
        if guild.is_none() {
            return;
        }

        // Filter channels for this guild
        let guild_channels: Vec<_> = channels
            .values()
            .filter(|c| c.guild_id == Some(guild_id))
            .collect();

        if guild_channels.is_empty() {
            ui.text_colored(DISCORD_TEXT_MUTED, "No channels");
            return;
        }

        // Group channels by category
        let mut categories: HashMap<Option<ChannelId>, Vec<&Channel>> = HashMap::new();
        for channel in guild_channels {
            categories.entry(channel.parent_id).or_default().push(channel);
        }

        // Sort categories (no parent first, then by position)
        let mut category_list: Vec<_> = categories.into_iter().collect();
        category_list.sort_by(|a, b| {
            match (a.0, b.0) {
                (None, Some(_)) => std::cmp::Ordering::Less,
                (Some(_), None) => std::cmp::Ordering::Greater,
                (None, None) => std::cmp::Ordering::Equal,
                (Some(a_id), Some(b_id)) => {
                    // Would need to look up category positions
                    a_id.0.cmp(&b_id.0)
                }
            }
        });

        // Search box
        let mut search_changed = false;
        ui.input_text("##ChannelSearch", &mut self.search_text)
            .hint("Search channels...")
            .flags(imgui::InputTextFlags::ENTER_RETURNS_TRUE)
            .build();
        // Note: In real impl, we'd filter by search_text

        ui.separator();

        // Render each category
        for (category_id, mut channels) in category_list {
            // Sort channels by position
            channels.sort_by_key(|c| c.position);

            // Category header
            let is_expanded = self.expanded_categories.get(&category_id.unwrap_or(ChannelId(0))).copied().unwrap_or(true);

            if let Some(cat_id) = category_id {
                if let Some(category) = channels.iter().find(|c| c.id == cat_id) {
                    let label = format!("{} {}", if is_expanded { "▼" } else { "▶" }, category.name.as_deref().unwrap_or("Category"));

                    if ui.selectable_config(label)
                        .flags(SelectableFlags::ALLOW_DOUBLE_CLICK | SelectableFlags::DONT_CLOSE_POPUPS)
                        .build() {
                        if ui.is_mouse_double_clicked(MouseButton::Left) {
                            self.expanded_categories.insert(cat_id, !is_expanded);
                        }
                    }
                }
            } else {
                // No category (top-level channels)
                ui.text_colored(DISCORD_TEXT_MUTED, "TEXT CHANNELS");
            }

            if is_expanded || category_id.is_none() {
                for channel in channels {
                    // Skip category channels themselves
                    if channel.kind == ChannelType::GuildCategory {
                        continue;
                    }

                    // Filter by search
                    if !self.search_text.is_empty() {
                        if let Some(name) = &channel.name {
                            if !name.to_lowercase().contains(&self.search_text.to_lowercase()) {
                                continue;
                            }
                        }
                    }

                    self.render_channel(ui, channel, selected);
                }
            }

            ui.spacing();
        }
    }

    /// Render a single channel
    fn render_channel(&self, ui: &Ui, channel: &Channel, selected: &SelectedItem) {
        let is_selected = matches!(selected, SelectedItem::Channel(id) if *id == channel.id);
        let is_muted = false; // Would check user's channel mute settings

        // Channel prefix based on type
        let prefix = match channel.kind {
            ChannelType::GuildText => "#",
            ChannelType::GuildVoice => "🔊 ",
            ChannelType::GuildStageVoice => "🎤 ",
            ChannelType::GuildForum => "📝 ",
            ChannelType::GuildAnnouncement => "📢 ",
            ChannelType::GuildAnnouncementThread => "📢 ",
            ChannelType::GuildPublicThread => "💬 ",
            ChannelType::GuildPrivateThread => "🔒 ",
            _ => "",
        };

        let name = channel.name.as_deref().unwrap_or("unknown");
        let label = format!("{}{}", prefix, name);

        let text_color = if is_selected {
            DISCORD_TEXT
        } else if is_muted {
            DISCORD_TEXT_MUTED
        } else {
            DISCORD_TEXT
        };

        let mut flags = SelectableFlags::ALLOW_DOUBLE_CLICK;
        if is_selected {
            flags |= SelectableFlags::SELECTED;
        }

        ui.push_style_color(imgui::StyleColor::Text, text_color);
        let clicked = ui.selectable_config(label)
            .flags(flags)
            .build();
        ui.pop_style_color();

        // Context menu
        if ui.begin_popup_context_item("##ChannelContext") {
            if ui.menu_item("Mute Channel") {
                // TODO
            }
            if ui.menu_item("Notification Settings") {
                // TODO
            }
            ui.separator();
            if ui.menu_item("Edit Channel") {
                // TODO
            }
            if ui.menu_item("Delete Channel") {
                // TODO
            }
            ui.end_popup();
        }

        // Unread indicator (placeholder)
        if false { // channel.has_unread
            ui.same_line();
            ui.text_colored(DISCORD_GREEN, "●");
        }
    }
}

impl Default for ChannelsWindow {
    fn default() -> Self {
        Self::new()
    }
}