//! Channel List Component
//!
//! Displays a list of channels with categories.

use imgui::{Ui, TreeNodeFlags};
use crate::theme::*;
use discord_common::discord_types::*;
use std::collections::HashMap;

/// Channel list component
pub struct ChannelList {
    /// Expanded categories
    expanded: HashMap<ChannelId, bool>,
    /// Search filter
    search: String,
}

impl ChannelList {
    pub fn new() -> Self {
        Self {
            expanded: HashMap::new(),
            search: String::new(),
        }
    }

    pub fn render(
        &mut self,
        ui: &Ui,
        channels: &HashMap<ChannelId, Channel>,
        guild_id: Option<GuildId>,
        selected: &Option<ChannelId>
    ) {
        // Filter channels for guild
        let guild_channels: Vec<_> = channels
            .values()
            .filter(|c| c.guild_id == guild_id)
            .collect();

        if guild_channels.is_empty() {
            ui.text_colored(DISCORD_TEXT_MUTED, "No channels");
            return;
        }

        // Group by category
        let mut categories: HashMap<Option<ChannelId>, Vec<&Channel>> = HashMap::new();
        for channel in guild_channels {
            categories.entry(channel.parent_id).or_default().push(channel);
        }

        // Sort
        let mut category_list: Vec<_> = categories.into_iter().collect();
        category_list.sort_by(|a, b| {
            match (a.0, b.0) {
                (None, Some(_)) => std::cmp::Ordering::Less,
                (Some(_), None) => std::cmp::Ordering::Greater,
                (None, None) => std::cmp::Ordering::Equal,
                (Some(a_id), Some(b_id)) => a_id.0.cmp(&b_id.0),
            }
        });

        // Search
        ui.input_text("##ChannelSearch", &mut self.search)
            .hint("Search...")
            .build();

        ui.separator();

        for (category_id, mut channels) in category_list {
            channels.sort_by_key(|c| c.position);

            // Category header
            if let Some(cat_id) = category_id {
                if let Some(category) = channels.iter().find(|c| c.id == cat_id) {
                    let is_expanded = self.expanded.get(&cat_id).copied().unwrap_or(true);
                    let label = format!("{} {}", if is_expanded { "▼" } else { "▶" },
                        category.name.as_deref().unwrap_or("Category"));

                    if ui.selectable_config(label)
                        .flags(imgui::SelectableFlags::ALLOW_DOUBLE_CLICK)
                        .build() {
                        if ui.is_mouse_double_clicked(imgui::MouseButton::Left) {
                            self.expanded.insert(cat_id, !is_expanded);
                        }
                    }
                }
            }

            let is_expanded = category_id.map(|id| self.expanded.get(&id).copied().unwrap_or(true)).unwrap_or(true);

            if is_expanded {
                for channel in channels {
                    if channel.kind == ChannelType::GuildCategory {
                        continue;
                    }

                    // Filter by search
                    if !self.search.is_empty() {
                        if let Some(name) = &channel.name {
                            if !name.to_lowercase().contains(&self.search.to_lowercase()) {
                                continue;
                            }
                        }
                    }

                    self.render_channel(ui, channel, selected);
                }
            }
        }
    }

    fn render_channel(&self, ui: &Ui, channel: &Channel, selected: &Option<ChannelId>) {
        let is_selected = selected.as_ref() == Some(&channel.id);

        let prefix = match channel.kind {
            ChannelType::GuildText => "#",
            ChannelType::GuildVoice => "🔊 ",
            ChannelType::GuildStageVoice => "🎤 ",
            ChannelType::GuildForum => "📝 ",
            ChannelType::GuildAnnouncement => "📢 ",
            _ => "",
        };

        let name = channel.name.as_deref().unwrap_or("unknown");
        let label = format!("{}{}", prefix, name);

        let mut flags = imgui::SelectableFlags::empty();
        if is_selected {
            flags |= imgui::SelectableFlags::SELECTED;
        }

        let color = if is_selected { DISCORD_ACCENT } else { DISCORD_TEXT };

        ui.push_style_color(imgui::StyleColor::Text, color);
        ui.selectable_config(label).flags(flags).build();
        ui.pop_style_color();

        // Unread badge
        if false { // channel.has_unread
            ui.same_line();
            ui.text_colored(DISCORD_GREEN, "●");
        }
    }
}

impl Default for ChannelList {
    fn default() -> Self {
        Self::new()
    }
}