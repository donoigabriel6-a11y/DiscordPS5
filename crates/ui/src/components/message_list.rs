//! Message List Component
//!
//! Displays a list of messages with author, content, timestamp, and reactions.

use imgui::{Ui, Clipper, TableFlags, TreeNodeFlags};
use crate::theme::*;
use discord_common::discord_types::*;
use std::collections::HashMap;

/// Message list component
pub struct MessageList {
    /// Show timestamps
    show_timestamps: bool,
    /// Compact mode
    compact_mode: bool,
    /// Group messages by author
    group_by_author: bool,
}

impl MessageList {
    pub fn new() -> Self {
        Self {
            show_timestamps: true,
            compact_mode: false,
            group_by_author: true,
        }
    }

    pub fn render(
        &mut self,
        ui: &Ui,
        messages: &[Message],
        users: &HashMap<UserId, User>,
        current_user: &Option<User>,
        channel_id: &ChannelId
    ) {
        if messages.is_empty() {
            ui.text_colored(DISCORD_TEXT_MUTED, "No messages in this channel");
            return;
        }

        // Use clipper for performance with many messages
        let clipper = Clipper::new(messages.len());

        for idx in clipper.iter() {
            let message = &messages[idx];
            self.render_message(ui, message, users, current_user);
        }
    }

    fn render_message(
        &self,
        ui: &Ui,
        message: &Message,
        users: &HashMap<UserId, User>,
        current_user: &Option<User>
    ) {
        let author = users.get(&message.author.id);
        let is_own = current_user.as_ref().map(|u| u.id == message.author.id).unwrap_or(false);

        // Check if we should group with previous message
        let show_author = true; // Would check previous message author

        // Message container
        let spacing = if self.compact_mode { 2.0 } else { 8.0 };
        ui.dummy([0.0, spacing]);

        if show_author {
            // Avatar
            ui.image(
                imgui::TextureId::from(0), // Placeholder for avatar
                [40.0, 40.0]
            );

            ui.same_line();

            // Author name and timestamp
            let name_color = if message.author.bot {
                DISCORD_YELLOW
            } else if is_own {
                DISCORD_ACCENT
            } else {
                DISCORD_TEXT
            };

            ui.text_colored(name_color, author.map(|u| u.username.as_str()).unwrap_or("Unknown"));

            if self.show_timestamps {
                ui.same_line();
                ui.text_colored(DISCORD_TEXT_MUTED, format_timestamp(message.timestamp));
            }

            // Bot badge
            if message.author.bot {
                ui.same_line();
                ui.text_colored(DISCORD_TEXT_MUTED, "BOT");
            }
        } else {
            // Indent for grouped messages
            ui.dummy([40.0 + 8.0, 0.0]);
            ui.same_line();
        }

        // Message content
        ui.text_wrapped(&message.content);

        // Attachments
        for attachment in &message.attachments {
            ui.separator();
            ui.text(format!("📎 {} ({:.1} KB)", attachment.filename, attachment.size as f32 / 1024.0));
        }

        // Embeds
        for embed in &message.embeds {
            self.render_embed(ui, embed);
        }

        // Reactions
        if !message.reactions.is_empty() {
            ui.separator();
            self.render_reactions(ui, &message.reactions, is_own);
        }

        // Context menu
        if ui.begin_popup_context_item(&format!("##MsgCtx{}", message.id)) {
            if ui.menu_item("Reply") {
                // TODO
            }
            if ui.menu_item("Copy Message Link") {
                // TODO
            }
            if ui.menu_item("Copy Text") {
                // TODO
            }
            ui.separator();
            if ui.menu_item("Edit") && is_own {
                // TODO
            }
            if ui.menu_item("Delete") && is_own {
                // TODO
            }
            ui.separator();
            if ui.menu_item("Mark Unread") {
                // TODO
            }
            if ui.menu_item("Pin") {
                // TODO
            }
            ui.end_popup();
        }
    }

    fn render_embed(&self, ui: &Ui, embed: &Embed) {
        ui.push_style_color(imgui::StyleColor::ChildBg, DISCORD_SIDEBAR_BG.into());
        ui.push_style_var(imgui::StyleVar::ChildRounding, 4.0);
        ui.push_style_var(imgui::StyleVar::ChildBorderSize, 1.0);

        ui.child_window(format!("##Embed{}", embed.url.as_deref().unwrap_or("")))
            .flags(imgui::WindowFlags::NO_SCROLLBAR)
            .build(|| {
                // Title
                if let Some(title) = &embed.title {
                    ui.text_colored(DISCORD_TEXT, title);
                }

                // Description
                if let Some(desc) = &embed.description {
                    ui.text_wrapped(desc);
                }

                // Fields
                for field in &embed.fields {
                    ui.text_colored(DISCORD_TEXT_MUTED, &field.name);
                    ui.text_wrapped(&field.value);
                }

                // Footer
                if let Some(footer) = &embed.footer {
                    ui.separator();
                    ui.text_colored(DISCORD_TEXT_MUTED, footer.text.as_deref().unwrap_or(""));
                }
            });

        ui.pop_style_var(2);
        ui.pop_style_color();
    }

    fn render_reactions(&self, ui: &Ui, reactions: &[Reaction], is_own: bool) {
        ui.push_style_var(imgui::StyleVar::ItemSpacing, [4.0, 2.0]);

        for reaction in reactions {
            let emoji = reaction.emoji.name.as_deref().unwrap_or("❓");
            let count = reaction.count;
            let me = reaction.me;

            let bg_color = if me {
                DISCORD_ACCENT
            } else {
                DISCORD_SIDEBAR_BG
            };

            let text_color = if me {
                [1.0, 1.0, 1.0, 1.0]
            } else {
                DISCORD_TEXT
            };

            ui.push_style_color(imgui::StyleColor::Button, bg_color.into());
            ui.push_style_color(imgui::StyleColor::ButtonHovered, DISCORD_HOVER.into());
            ui.push_style_color(imgui::StyleColor::ButtonActive, DISCORD_ACTIVE.into());
            ui.push_style_color(imgui::StyleColor::Text, text_color.into());

            let label = format!("{} {}", emoji, count);
            if ui.small_button(&label) {
                // Toggle reaction
            }

            ui.pop_style_color(4);
            ui.same_line();
        }

        // Add reaction button
        if ui.small_button("➕") {
            // Show emoji picker
        }

        ui.pop_style_var();
    }
}

fn format_timestamp(timestamp: chrono::DateTime<chrono::Utc>) -> String {
    let now = chrono::Utc::now();
    let diff = now - timestamp;

    if diff.num_days() > 0 {
        timestamp.format("%b %d").to_string()
    } else if diff.num_hours() > 0 {
        format!("{}h ago", diff.num_hours())
    } else if diff.num_minutes() > 0 {
        format!("{}m ago", diff.num_minutes())
    } else {
        "now".to_string()
    }
}

impl Default for MessageList {
    fn default() -> Self {
        Self::new()
    }
}