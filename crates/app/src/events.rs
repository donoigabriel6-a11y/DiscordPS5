//! Event Handler
//!
//! Handles Discord gateway events and dispatches them to the application.

use std::sync::Arc;
use tokio::sync::mpsc;
use tracing::{debug, error, info, warn};
use discord_common::discord_types::*;
use discord_gateway::GatewayEvent;
use discord_rest::RestClient;
use crate::state::{AppState, SelectedItem};

/// Event handler for gateway events
pub struct EventHandler {
    app_state: Arc<AppState>,
}

impl EventHandler {
    pub fn new(app_state: Arc<AppState>) -> Self {
        Self { app_state }
    }

    /// Process gateway events
    pub async fn process_gateway_events(&mut self, app: &mut crate::App) {
        // In a real implementation, we'd process events from the gateway
        // For now, the App handles this directly in its update loop
    }

    /// Handle a single gateway event
    pub async fn handle_event(&self, event: GatewayEvent) {
        use GatewayEvent::*;

        match event {
            Ready(data) => self.handle_ready(data).await,
            Resumed => self.handle_resumed().await,
            GuildCreate(guild) => self.handle_guild_create(guild).await,
            GuildUpdate(guild) => self.handle_guild_update(guild).await,
            GuildDelete(unavailable) => self.handle_guild_delete(unavailable).await,
            ChannelCreate(channel) => self.handle_channel_create(channel).await,
            ChannelUpdate(channel) => self.handle_channel_update(channel).await,
            ChannelDelete(channel) => self.handle_channel_delete(channel).await,
            ChannelPinsUpdate(pins) => self.handle_channel_pins_update(pins).await,
            MessageCreate(msg) => self.handle_message_create(msg).await,
            MessageUpdate(msg) => self.handle_message_update(msg).await,
            MessageDelete(delete) => self.handle_message_delete(delete).await,
            MessageDeleteBulk(bulk) => self.handle_message_delete_bulk(bulk).await,
            MessageReactionAdd(reaction) => self.handle_reaction_add(reaction).await,
            MessageReactionRemove(reaction) => self.handle_reaction_remove(reaction).await,
            MessageReactionRemoveAll(message_id, channel_id) => self.handle_reaction_remove_all(message_id, channel_id).await,
            MessageReactionRemoveEmoji(emoji, channel_id) => self.handle_reaction_remove_emoji(emoji, channel_id).await,
            VoiceStateUpdate(state) => self.handle_voice_state_update(state).await,
            VoiceServerUpdate(update) => self.handle_voice_server_update(update).await,
            PresenceUpdate(presence) => self.handle_presence_update(presence).await,
            UserUpdate(user) => self.handle_user_update(user).await,
            TypingStart(typing) => self.handle_typing_start(typing).await,
            _ => {}
        }
    }

    async fn handle_ready(&self, data: ReadyEvent) {
        info!("Gateway READY: {} guilds, {} users", data.guilds.len(), data.users.len());

        // Update guilds
        for guild in data.guilds {
            self.app_state.guilds.write().insert(guild.id, guild);
        }

        // Update users
        for user in data.users {
            self.app_state.users.write().insert(user.id, user);
        }

        // Update private channels (DMs)
        for channel in data.private_channels {
            self.app_state.channels.write().insert(channel.id, channel);
        }

        // Set current user
        if let Some(user) = data.user {
            *self.app_state.current_user.write() = Some(user.clone());
            self.app_state.users.write().insert(user.id, user);
        }

        // Store session info for resume
        self.app_state.config.gateway.session_id = Some(data.session_id);
    }

    async fn handle_resumed(&self) {
        info!("Gateway session resumed");
    }

    async fn handle_guild_create(&self, guild: Guild) {
        debug!("Guild created: {} ({})", guild.name, guild.id);
        self.app_state.guilds.write().insert(guild.id, guild);
    }

    async fn handle_guild_update(&self, guild: Guild) {
        debug!("Guild updated: {} ({})", guild.name, guild.id);
        self.app_state.guilds.write().insert(guild.id, guild);
    }

    async fn handle_guild_delete(&self, unavailable: UnavailableGuild) {
        if unavailable.unavailable {
            warn!("Guild unavailable: {}", unavailable.id);
        } else {
            info!("Guild deleted: {}", unavailable.id);
            self.app_state.guilds.write().remove(&unavailable.id);
            self.app_state.channels.write().retain(|_, c| c.guild_id != Some(unavailable.id));
        }
    }

    async fn handle_channel_create(&self, channel: Channel) {
        debug!("Channel created: {:?} ({})", channel.kind, channel.id);
        self.app_state.channels.write().insert(channel.id, channel);
    }

    async fn handle_channel_update(&self, channel: Channel) {
        debug!("Channel updated: {:?} ({})", channel.kind, channel.id);
        self.app_state.channels.write().insert(channel.id, channel);
    }

    async fn handle_channel_delete(&self, channel: Channel) {
        debug!("Channel deleted: {:?} ({})", channel.kind, channel.id);
        self.app_state.channels.write().remove(&channel.id);
        self.app_state.messages.write().remove(&channel.id);
    }

    async fn handle_channel_pins_update(&self, pins: ChannelPinsUpdateEvent) {
        debug!("Channel pins updated: {}", pins.channel_id);
        // Would update pinned messages
    }

    async fn handle_message_create(&self, message: Message) {
        debug!("Message created in channel {}", message.channel_id);
        self.app_state.add_message(message);
    }

    async fn handle_message_update(&self, message: Message) {
        debug!("Message updated: {}", message.id);
        self.app_state.update_message(message);
    }

    async fn handle_message_delete(&self, delete: MessageDeleteEvent) {
        debug!("Message deleted: {} in channel {}", delete.id, delete.channel_id);
        self.app_state.remove_message(delete.channel_id, delete.id);
    }

    async fn handle_message_delete_bulk(&self, bulk: MessageDeleteBulkEvent) {
        debug!("Bulk message delete in channel {}: {} messages", bulk.channel_id, bulk.ids.len());
        if let Some(messages) = self.app_state.messages.write().get_mut(&bulk.channel_id) {
            messages.retain(|m| !bulk.ids.contains(&m.id));
        }
    }

    async fn handle_reaction_add(&self, reaction: MessageReactionAddEvent) {
        debug!("Reaction added: {} on message {}", reaction.emoji.name(), reaction.message_id);
        // Would update message reactions
    }

    async fn handle_reaction_remove(&self, reaction: MessageReactionRemoveEvent) {
        debug!("Reaction removed: {} on message {}", reaction.emoji.name(), reaction.message_id);
    }

    async fn handle_reaction_remove_all(&self, message_id: MessageId, channel_id: ChannelId) {
        debug!("All reactions removed from message {}", message_id);
    }

    async fn handle_reaction_remove_emoji(&self, emoji: Emoji, channel_id: ChannelId) {
        debug!("Emoji reactions removed: {} in channel {}", emoji.name(), channel_id);
    }

    async fn handle_voice_state_update(&self, state: VoiceState) {
        debug!("Voice state update: user {} in guild {}", state.user_id, state.guild_id);
        self.app_state.voice_states.write()
            .entry(state.guild_id)
            .or_default()
            .insert(state.user_id, state);
    }

    async fn handle_voice_server_update(&self, update: VoiceServerUpdateEvent) {
        info!("Voice server update for guild {}: endpoint={}", update.guild_id, update.endpoint);
        // Would update voice connection with new server info
        if let Some(voice) = self.app_state.voice_connection() {
            // voice.update_server(update.token, update.endpoint).await;
        }
    }

    async fn handle_presence_update(&self, presence: PresenceUpdateEvent) {
        debug!("Presence update: user {}", presence.user.id);
        self.app_state.users.write().insert(presence.user.id, presence.user);
        // Would update guild member presence
    }

    async fn handle_user_update(&self, user: User) {
        debug!("User updated: {}", user.id);
        self.app_state.users.write().insert(user.id, user);
    }

    async fn handle_typing_start(&self, typing: TypingStartEvent) {
        debug!("Typing start: user {} in channel {}", typing.user_id, typing.channel_id);
        // Would show typing indicator
    }
}