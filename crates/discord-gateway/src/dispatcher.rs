//! Event Dispatcher for Gateway Events

use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use std::any::Any;

use anyhow::{Context, Result};
use parking_lot::RwLock;
use tracing::{debug, error, info, warn};

use crate::{GatewayEvent, DiscordError, Result as GatewayResult};

/// Event handler trait
pub trait EventHandler: Send + Sync {
    fn handle(&self, event: &GatewayEvent);
    fn event_type(&self) -> &'static str;
}

/// Boxed event handler
type BoxedHandler = Box<dyn EventHandler>;

/// Event dispatcher for routing gateway events to handlers
pub struct EventDispatcher {
    handlers: Arc<RwLock<HashMap<String, Vec<BoxedHandler>>>>,
    global_handlers: Arc<Mutex<Vec<BoxedHandler>>>,
}

impl EventDispatcher {
    pub fn new() -> Self {
        Self {
            handlers: Arc::new(RwLock::new(HashMap::new())),
            global_handlers: Arc::new(Mutex::new(Vec::new())),
        }
    }

    /// Register a handler for a specific event type
    pub fn register<H>(&self, event_type: &str, handler: H)
    where
        H: EventHandler + 'static,
    {
        self.handlers.write().entry(event_type.to_string())
            .or_default()
            .push(Box::new(handler));
        info!("Registered handler for event: {}", event_type);
    }

    /// Register a global handler that receives all events
    pub fn register_global<H>(&self, handler: H)
    where
        H: EventHandler + 'static,
    {
        self.global_handlers.lock().unwrap().push(Box::new(handler));
        info!("Registered global handler");
    }

    /// Dispatch an event to all registered handlers
    pub fn dispatch(&self, event: &GatewayEvent) {
        let event_name = Self::event_name(event);

        // Call specific handlers
        if let Some(handlers) = self.handlers.read().get(&event_name) {
            for handler in handlers {
                handler.handle(event);
            }
        }

        // Call global handlers
        for handler in self.global_handlers.lock().unwrap().iter() {
            handler.handle(event);
        }
    }

    /// Get event name from GatewayEvent
    fn event_name(event: &GatewayEvent) -> String {
        match event {
            GatewayEvent::Ready(_) => "READY",
            GatewayEvent::Resumed => "RESUMED",
            GatewayEvent::MessageCreate(_) => "MESSAGE_CREATE",
            GatewayEvent::MessageUpdate(_) => "MESSAGE_UPDATE",
            GatewayEvent::MessageDelete(_) => "MESSAGE_DELETE",
            GatewayEvent::MessageDeleteBulk(_) => "MESSAGE_DELETE_BULK",
            GatewayEvent::ChannelCreate(_) => "CHANNEL_CREATE",
            GatewayEvent::ChannelUpdate(_) => "CHANNEL_UPDATE",
            GatewayEvent::ChannelDelete(_) => "CHANNEL_DELETE",
            GatewayEvent::ChannelPinsUpdate(_) => "CHANNEL_PINS_UPDATE",
            GatewayEvent::GuildCreate(_) => "GUILD_CREATE",
            GatewayEvent::GuildUpdate(_) => "GUILD_UPDATE",
            GatewayEvent::GuildDelete(_) => "GUILD_DELETE",
            GatewayEvent::GuildMemberAdd(_) => "GUILD_MEMBER_ADD",
            GatewayEvent::GuildMemberUpdate(_) => "GUILD_MEMBER_UPDATE",
            GatewayEvent::GuildMemberRemove(_) => "GUILD_MEMBER_REMOVE",
            GatewayEvent::GuildMembersChunk(_) => "GUILD_MEMBERS_CHUNK",
            GatewayEvent::GuildRoleCreate(_) => "GUILD_ROLE_CREATE",
            GatewayEvent::GuildRoleUpdate(_) => "GUILD_ROLE_UPDATE",
            GatewayEvent::GuildRoleDelete(_) => "GUILD_ROLE_DELETE",
            GatewayEvent::GuildBanAdd(_) => "GUILD_BAN_ADD",
            GatewayEvent::GuildBanRemove(_) => "GUILD_BAN_REMOVE",
            GatewayEvent::GuildEmojisUpdate(_) => "GUILD_EMOJIS_UPDATE",
            GatewayEvent::GuildStickersUpdate(_) => "GUILD_STICKERS_UPDATE",
            GatewayEvent::GuildIntegrationsUpdate(_) => "GUILD_INTEGRATIONS_UPDATE",
            GatewayEvent::GuildWebhooksUpdate(_) => "GUILD_WEBHOOKS_UPDATE",
            GatewayEvent::InviteCreate(_) => "INVITE_CREATE",
            GatewayEvent::InviteDelete(_) => "INVITE_DELETE",
            GatewayEvent::ThreadCreate(_) => "THREAD_CREATE",
            GatewayEvent::ThreadUpdate(_) => "THREAD_UPDATE",
            GatewayEvent::ThreadDelete(_) => "THREAD_DELETE",
            GatewayEvent::ThreadListSync(_) => "THREAD_LIST_SYNC",
            GatewayEvent::ThreadMemberUpdate(_) => "THREAD_MEMBER_UPDATE",
            GatewayEvent::ThreadMembersUpdate(_) => "THREAD_MEMBERS_UPDATE",
            GatewayEvent::VoiceStateUpdate(_) => "VOICE_STATE_UPDATE",
            GatewayEvent::VoiceServerUpdate(_) => "VOICE_SERVER_UPDATE",
            GatewayEvent::PresenceUpdate(_) => "PRESENCE_UPDATE",
            GatewayEvent::UserUpdate(_) => "USER_UPDATE",
            GatewayEvent::InteractionCreate(_) => "INTERACTION_CREATE",
            GatewayEvent::ApplicationCommandCreate(_) => "APPLICATION_COMMAND_CREATE",
            GatewayEvent::ApplicationCommandUpdate(_) => "APPLICATION_COMMAND_UPDATE",
            GatewayEvent::ApplicationCommandDelete(_) => "APPLICATION_COMMAND_DELETE",
            GatewayEvent::Raw(raw) => &raw.event_type,
        }.to_string()
    }

    /// Clear all handlers
    pub fn clear(&self) {
        self.handlers.write().clear();
        self.global_handlers.lock().unwrap().clear();
    }
}

impl Default for EventDispatcher {
    fn default() -> Self {
        Self::new()
    }
}

/// Macro for creating simple event handlers
#[macro_export]
macro_rules! event_handler {
    ($event_type:expr, |$event:ident| $body:expr) => {
        struct Handler;
        impl EventHandler for Handler {
            fn handle(&self, $event: &GatewayEvent) {
                $body
            }
            fn event_type(&self) -> &'static str {
                $event_type
            }
        }
        Handler
    };
}