//! Member List Component
//!
//! Displays the list of members in a server/voice channel.

use imgui::{Ui, Clipper};
use crate::theme::*;
use discord_common::discord_types::*;
use std::collections::HashMap;

/// Member list component
pub struct MemberList {
    /// Show offline members
    show_offline: bool,
    /// Search filter
    search: String,
    /// Sort by: 0 = name, 1 = status, 2 = role
    sort_mode: u8,
}

impl MemberList {
    pub fn new() -> Self {
        Self {
            show_offline: false,
            search: String::new(),
            sort_mode: 0,
        }
    }

    pub fn render(
        &mut self,
        ui: &Ui,
        voice_states: &HashMap<GuildId, HashMap<UserId, VoiceState>>,
        guilds: &HashMap<GuildId, Guild>,
        users: &HashMap<UserId, User>,
        selected: &Option<GuildId>
    ) {
        // Get selected guild
        let guild_id = match selected {
            Some(GuildId(id)) if *id != 0 => *id,
            _ => {
                ui.text_colored(DISCORD_TEXT_MUTED, "Select a server to view members");
                return;
            }
        };

        let guild = guilds.get(&GuildId(guild_id));
        if guild.is_none() {
            return;
        }

        // Get voice states for this guild
        let guild_voice_states = voice_states.get(&GuildId(guild_id));

        // Search
        ui.input_text("##MemberSearch", &mut self.search)
            .hint("Search members...")
            .build();

        ui.separator();

        // Member count
        ui.text_colored(DISCORD_TEXT_MUTED, "Members");
        ui.same_line();
        ui.text_colored(DISCORD_TEXT_MUTED, "0"); // Would count actual members

        ui.separator();

        // In a real implementation, we'd get the member list from the guild
        // For now, show voice-connected users
        if let Some(vstates) = guild_voice_states {
            if !vstates.is_empty() {
                ui.text_colored(DISCORD_GREEN, "🔊 Voice Connected");

                let members: Vec<_> = vstates.keys().collect();
                let clipper = Clipper::new(members.len());

                for idx in clipper.iter() {
                    let user_id = members[idx];
                    if let Some(user) = users.get(user_id) {
                        self.render_member(ui, user, true);
                    }
                }
            }
        }

        // Offline/other members would go here
        if self.show_offline {
            ui.text_colored(DISCORD_TEXT_MUTED, "Offline members hidden");
        }
    }

    fn render_member(&self, ui: &Ui, user: &User, in_voice: bool) {
        // Avatar
        ui.image(
            imgui::TextureId::from(0), // Placeholder
            [24.0, 24.0]
        );

        ui.same_line();

        // Status indicator
        let status_color = if in_voice {
            DISCORD_GREEN
        } else {
            DISCORD_TEXT_MUTED
        };

        let cursor = ui.cursor_screen_pos();
        ui.get_window_draw_list().add_circle_filled(
            [cursor[0] + 10.0, cursor[1] + 10.0],
            5.0,
            status_color.into()
        ).build();

        // Name
        ui.dummy([20.0, 0.0]);
        ui.same_line();
        ui.text(&user.username);

        // Bot badge
        if user.bot {
            ui.same_line();
            ui.text_colored(DISCORD_YELLOW, "BOT");
        }
    }
}

impl Default for MemberList {
    fn default() -> Self {
        Self::new()
    }
}