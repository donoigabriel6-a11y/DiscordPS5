//! Messages Window (Main Content Area)
//!
//! Displays messages for the selected channel.

use imgui::{Ui, TableFlags, SelectableFlags, MouseButton, Clipper};
use crate::theme::*;
use crate::components::MessageList;
use discord_common::discord_types::*;
use std::collections::HashMap;

/// Messages window
pub struct MessagesWindow {
    /// Message list component
    message_list: MessageList,
    /// Auto-scroll to bottom
    auto_scroll: bool,
    /// Show thread indicator
    show_threads: bool,
}

impl MessagesWindow {
    pub fn new() -> Self {
        Self {
            message_list: MessageList::new(),
            auto_scroll: true,
            show_threads: true,
        }
    }

    pub fn render(
        &mut self,
        ui: &Ui,
        messages: &HashMap<ChannelId, Vec<Message>>,
        channels: &HashMap<ChannelId, Channel>,
        users: &HashMap<UserId, User>,
        selected: &SelectedItem,
        current_user: &Option<User>
    ) {
        // Get selected channel
        let channel_id = match selected {
            SelectedItem::Channel(id) => *id,
            SelectedItem::DMChannel(id) => *id,
            _ => {
                ui.text_colored(DISCORD_TEXT_MUTED, "Select a channel to view messages");
                return;
            }
        };

        let channel = channels.get(&channel_id);
        let channel_messages = messages.get(&channel_id);

        // Channel header
        if let Some(channel) = channel {
            self.render_channel_header(ui, channel);
        }

        ui.separator();

        // Messages list
        if let Some(messages) = channel_messages {
            self.message_list.render(
                ui,
                messages,
                users,
                current_user,
                &channel_id
            );
        } else {
            ui.text_colored(DISCORD_TEXT_MUTED, "No messages yet");
        }

        // Handle auto-scroll
        if self.auto_scroll {
            ui.set_scroll_here_y(1.0);
        }
    }

    /// Render channel header with topic, pins, etc.
    fn render_channel_header(&self, ui: &Ui, channel: &Channel) {
        ui.text(channel.name.as_deref().unwrap_or("unknown"));

        if let Some(topic) = &channel.topic {
            if !topic.is_empty() {
                ui.same_line();
                ui.text_colored(DISCORD_TEXT_MUTED, format!("• {}", topic));
            }
        }

        // Pinned messages button (if any)
        if let Some(pins) = &channel.pinned_message_ids {
            if !pins.is_empty() {
                ui.same_line();
                if ui.small_button("📌 Pins") {
                    // TODO: Show pins popup
                }
            }
        }

        // Threads button
        if self.show_threads {
            ui.same_line();
            if ui.small_button("🧵 Threads") {
                // TODO: Show threads popup
            }
        }
    }
}

impl Default for MessagesWindow {
    fn default() -> Self {
        Self::new()
    }
}