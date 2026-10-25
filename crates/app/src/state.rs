//! Application State Management
//!
//! Centralized state management for the Discord PS5 client.

use std::sync::Arc;
use std::collections::HashMap;
use parking_lot::RwLock;
use tokio::sync::mpsc;
use discord_common::{config::Config, discord_types::*};
use discord_gateway::GatewayEvent;
use discord_rest::RestClient;
use discord_voice::{VoiceConnection, VoiceConfig, VoiceEvent, VoiceConnectionState};

/// Shared application state
pub struct AppState {
    /// Configuration
    pub config: Config,

    /// Current user
    pub current_user: Arc<RwLock<Option<User>>>,

    /// Guilds
    pub guilds: Arc<RwLock<HashMap<GuildId, Guild>>>,

    /// Channels
    pub channels: Arc<RwLock<HashMap<ChannelId, Channel>>>,

    /// Users
    pub users: Arc<RwLock<HashMap<UserId, User>>>,

    /// Messages (per channel)
    pub messages: Arc<RwLock<HashMap<ChannelId, Vec<Message>>>>,

    /// Voice states (per guild)
    pub voice_states: Arc<RwLock<HashMap<GuildId, HashMap<UserId, VoiceState>>>>,

    /// Selected item in UI
    pub selected: Arc<RwLock<SelectedItem>>,

    /// Voice connection state
    pub voice_state: Arc<RwLock<VoiceConnectionState>>,

    /// Current voice channel
    pub voice_channel_id: Arc<RwLock<Option<ChannelId>>>,

    /// Voice connection
    pub voice_connection: Arc<RwLock<Option<VoiceConnection>>>,

    /// Voice events channel
    pub voice_events_tx: mpsc::UnboundedSender<VoiceEvent>,
    pub voice_events_rx: Arc<RwLock<Option<mpsc::UnboundedReceiver<VoiceEvent>>>>,

    /// Gateway events channel
    pub gateway_events_tx: mpsc::UnboundedSender<GatewayEvent>,
    pub gateway_events_rx: Arc<RwLock<Option<mpsc::UnboundedReceiver<GatewayEvent>>>>,

    /// REST client
    pub rest_client: Arc<RwLock<Option<RestClient>>>,

    /// Settings
    pub settings: Arc<RwLock<UserSettings>>,

    /// Running state
    pub running: Arc<RwLock<bool>>,
}

/// Selected item in UI
#[derive(Debug, Clone, PartialEq)]
pub enum SelectedItem {
    None,
    Server(GuildId),
    Channel(ChannelId),
    User(UserId),
    DMChannel(ChannelId),
}

impl Default for SelectedItem {
    fn default() -> Self {
        SelectedItem::None
    }
}

/// User settings
#[derive(Debug, Clone, Default)]
pub struct UserSettings {
    /// Theme: 0 = dark, 1 = light, 2 = system
    pub theme: u8,

    /// Compact mode
    pub compact_mode: bool,

    /// Show timestamps
    pub show_timestamps: bool,

    /// Font scale (percentage)
    pub font_scale: u8,

    /// Zoom level (percentage)
    pub zoom_level: u8,

    /// Voice input volume (0-200)
    pub voice_input_volume: u8,

    /// Voice output volume (0-200)
    pub voice_output_volume: u8,

    /// Input mode: 0 = voice activity, 1 = push to talk
    pub input_mode: u8,

    /// Push to talk key
    pub push_to_talk_key: Option<String>,

    /// Noise suppression
    pub noise_suppression: bool,

    /// Echo cancellation
    pub echo_cancellation: bool,

    /// Automatic gain control
    pub agc: bool,

    /// Enable notifications
    pub enable_notifications: bool,

    /// Notification sound
    pub notification_sound: bool,

    /// Developer mode
    pub developer_mode: bool,

    /// Hardware acceleration
    pub hardware_acceleration: bool,
}

impl AppState {
    /// Create new application state
    pub fn new(config: Config) -> Self {
        let (voice_tx, voice_rx) = mpsc::unbounded_channel();
        let (gateway_tx, gateway_rx) = mpsc::unbounded_channel();

        Self {
            config,
            current_user: Arc::new(RwLock::new(None)),
            guilds: Arc::new(RwLock::new(HashMap::new())),
            channels: Arc::new(RwLock::new(HashMap::new())),
            users: Arc::new(RwLock::new(HashMap::new())),
            messages: Arc::new(RwLock::new(HashMap::new())),
            voice_states: Arc::new(RwLock::new(HashMap::new())),
            selected: Arc::new(RwLock::new(SelectedItem::None)),
            voice_state: Arc::new(RwLock::new(VoiceConnectionState::Disconnected)),
            voice_channel_id: Arc::new(RwLock::new(None)),
            voice_connection: Arc::new(RwLock::new(None)),
            voice_events_tx: voice_tx,
            voice_events_rx: Arc::new(RwLock::new(Some(voice_rx))),
            gateway_events_tx: gateway_tx,
            gateway_events_rx: Arc::new(RwLock::new(Some(gateway_rx))),
            rest_client: Arc::new(RwLock::new(None)),
            settings: Arc::new(RwLock::new(UserSettings::default())),
            running: Arc::new(RwLock::new(true)),
        }
    }

    /// Get voice events receiver
    pub fn take_voice_events(&self) -> Option<mpsc::UnboundedReceiver<VoiceEvent>> {
        self.voice_events_rx.write().take()
    }

    /// Get gateway events receiver
    pub fn take_gateway_events(&self) -> Option<mpsc::UnboundedReceiver<GatewayEvent>> {
        self.gateway_events_rx.write().take()
    }

    /// Check if running
    pub fn is_running(&self) -> bool {
        *self.running.read()
    }

    /// Set running state
    pub fn set_running(&self, running: bool) {
        *self.running.write() = running;
    }

    /// Add a message
    pub fn add_message(&self, message: Message) {
        self.messages.write()
            .entry(message.channel_id)
            .or_default()
            .push(message);
    }

    /// Update a message
    pub fn update_message(&self, message: Message) {
        if let Some(messages) = self.messages.write().get_mut(&message.channel_id) {
            if let Some(idx) = messages.iter().position(|m| m.id == message.id) {
                messages[idx] = message;
            }
        }
    }

    /// Remove a message
    pub fn remove_message(&self, channel_id: ChannelId, message_id: MessageId) {
        if let Some(messages) = self.messages.write().get_mut(&channel_id) {
            messages.retain(|m| m.id != message_id);
        }
    }

    /// Set voice connection
    pub fn set_voice_connection(&self, conn: VoiceConnection) {
        *self.voice_connection.write() = Some(conn);
    }

    /// Get voice connection
    pub fn voice_connection(&self) -> Option<VoiceConnection> {
        self.voice_connection.read().clone()
    }
}