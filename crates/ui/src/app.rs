//! Main UI Application
//!
//! ImGui-based Discord UI application state and main loop.

use std::sync::Arc;
use std::collections::HashMap;
use std::time::{Duration, Instant};
use anyhow::{Context, Result};
use parking_lot::RwLock;
use tokio::sync::mpsc;
use tracing::{debug, error, info, warn};

use imgui::{Ui, Window, TableFlags, TreeNodeFlags, SelectableFlags};
use crate::theme::*;
use crate::renderer::Renderer;
use crate::windows::{ServersWindow, ChannelsWindow, MessagesWindow, VoiceWindow, SettingsWindow, UserMenuWindow};
use crate::components::{MessageList, ChannelList, MemberList, VoiceControls};
use discord_common::discord_types::*;
use discord_gateway::{GatewayClient, GatewayEvent};
use discord_rest::RestClient;
use discord_voice::{VoiceConnection, VoiceConfig, VoiceEvent, VoiceConnectionState};
use ps5_platform::input::{DualSense, InputEvent};

/// Application state
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AppState {
    /// Starting up, connecting to gateway
    Starting,
    /// Logging in / authenticating
    Authenticating,
    /// Connected and ready
    Ready,
    /// Reconnecting after disconnect
    Reconnecting,
    /// Error state
    Error,
    /// Shutting down
    Shutdown,
}

/// Selected item in the UI
#[derive(Debug, Clone, PartialEq)]
pub enum SelectedItem {
    None,
    Server(GuildId),
    Channel(ChannelId),
    User(UserId),
    DMChannel(ChannelId),
}

/// Main UI application
pub struct App {
    // State
    state: Arc<RwLock<AppState>>,
    selected: Arc<RwLock<SelectedItem>>,

    // Discord clients
    gateway: Option<GatewayClient>,
    rest: Option<RestClient>,
    voice: Option<VoiceConnection>,

    // Data
    guilds: Arc<RwLock<HashMap<GuildId, Guild>>>,
    channels: Arc<RwLock<HashMap<ChannelId, Channel>>>,
    users: Arc<RwLock<HashMap<UserId, User>>>,
    messages: Arc<RwLock<HashMap<ChannelId, Vec<Message>>>>,
    voice_states: Arc<RwLock<HashMap<GuildId, HashMap<UserId, VoiceState>>>>,

    // Current user
    current_user: Arc<RwLock<Option<User>>>,

    // Voice
    voice_channel_id: Arc<RwLock<Option<ChannelId>>>,
    voice_state: Arc<RwLock<VoiceConnectionState>>,
    voice_events: mpsc::UnboundedReceiver<VoiceEvent>,

    // UI windows
    servers_window: ServersWindow,
    channels_window: ChannelsWindow,
    messages_window: MessagesWindow,
    voice_window: VoiceWindow,
    settings_window: SettingsWindow,
    user_menu_window: UserMenuWindow,

    // UI components
    message_list: MessageList,
    channel_list: ChannelList,
    member_list: MemberList,
    voice_controls: VoiceControls,

    // UI state
    show_settings: bool,
    show_user_menu: bool,
    show_voice_popout: bool,
    settings_tab: SettingsTab,

    // Input
    dualsense: Option<DualSense>,
    last_input: Instant,

    // Performance
    frame_times: Vec<f32>,
    last_fps_update: Instant,
    current_fps: f32,
}

/// Settings tabs
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SettingsTab {
    Account,
    Voice,
    Video,
    Notifications,
    Keybinds,
    Appearance,
    Advanced,
}

impl Default for SettingsTab {
    fn default() -> Self {
        SettingsTab::Account
    }
}

impl App {
    /// Create a new application instance
    pub fn new() -> Result<Self> {
        let (voice_tx, voice_rx) = mpsc::unbounded_channel();

        Ok(Self {
            state: Arc::new(RwLock::new(AppState::Starting)),
            selected: Arc::new(RwLock::new(SelectedItem::None)),
            gateway: None,
            rest: None,
            voice: None,
            guilds: Arc::new(RwLock::new(HashMap::new())),
            channels: Arc::new(RwLock::new(HashMap::new())),
            users: Arc::new(RwLock::new(HashMap::new())),
            messages: Arc::new(RwLock::new(HashMap::new())),
            voice_states: Arc::new(RwLock::new(HashMap::new())),
            current_user: Arc::new(RwLock::new(None)),
            voice_channel_id: Arc::new(RwLock::new(None)),
            voice_state: Arc::new(RwLock::new(VoiceConnectionState::Disconnected)),
            voice_events: voice_rx,
            servers_window: ServersWindow::new(),
            channels_window: ChannelsWindow::new(),
            messages_window: MessagesWindow::new(),
            voice_window: VoiceWindow::new(),
            settings_window: SettingsWindow::new(),
            user_menu_window: UserMenuWindow::new(),
            message_list: MessageList::new(),
            channel_list: ChannelList::new(),
            member_list: MemberList::new(),
            voice_controls: VoiceControls::new(),
            show_settings: false,
            show_user_menu: false,
            show_voice_popout: false,
            settings_tab: SettingsTab::default(),
            dualsense: None,
            last_input: Instant::now(),
            frame_times: Vec::with_capacity(120),
            last_fps_update: Instant::now(),
            current_fps: 0.0,
        })
    }

    /// Initialize the application with Discord clients
    pub async fn initialize(
        &mut self,
        gateway: GatewayClient,
        rest: RestClient,
        voice_config: VoiceConfig,
    ) -> Result<()> {
        info!("Initializing Discord PS5 application...");

        // Store clients
        self.gateway = Some(gateway);
        self.rest = Some(rest);

        // Initialize voice connection
        let (voice_conn, voice_events) = VoiceConnection::new(voice_config);
        self.voice = Some(voice_conn);
        self.voice_events = voice_events;

        // Initialize DualSense
        if let Ok(dualsense) = DualSense::new() {
            self.dualsense = Some(dualsense);
            info!("DualSense controller connected");
        } else {
            warn!("DualSense controller not found");
        }

        // Load initial data
        self.load_initial_data().await?;

        *self.state.write() = AppState::Ready;
        info!("Application initialized successfully");

        Ok(())
    }

    /// Load initial data from Discord
    async fn load_initial_data(&mut self) -> Result<()> {
        let rest = self.rest.as_ref()
            .ok_or_else(|| anyhow::anyhow!("REST client not initialized"))?;

        // Get current user
        let current_user = rest.get_current_user().await?;
        *self.current_user.write() = Some(current_user.clone());
        self.users.write().insert(current_user.id, current_user);

        // Get guilds
        let guilds = rest.get_user_guilds().await?;
        for guild in guilds {
            self.guilds.write().insert(guild.id, guild);
        }

        info!("Loaded {} guilds", self.guilds.read().len());

        Ok(())
    }

    /// Main update loop
    pub fn update(&mut self, renderer: &mut Renderer, delta_time: f32) -> Result<bool> {
        // Update frame timing
        self.frame_times.push(delta_time);
        if self.frame_times.len() > 120 {
            self.frame_times.remove(0);
        }

        // Update FPS counter
        if self.last_fps_update.elapsed() >= Duration::from_secs(1) {
            self.current_fps = self.frame_times.len() as f32 /
                self.frame_times.iter().sum::<f32>().max(0.001);
            self.last_fps_update = Instant::now();
        }

        // Process voice events
        self.process_voice_events();

        // Process gateway events
        self.process_gateway_events();

        // Handle input
        self.handle_input();

        // Create ImGui frame
        let mut ui = renderer.new_frame()?;

        // Render UI
        self.render_ui(&mut ui)?;

        // Render frame
        renderer.render(ui)?;

        // Check for shutdown
        Ok(!matches!(*self.state.read(), AppState::Shutdown))
    }

    /// Process voice events
    fn process_voice_events(&mut self) {
        while let Ok(event) = self.voice_events.try_recv() {
            match event {
                VoiceEvent::Connected => {
                    *self.voice_state.write() = VoiceConnectionState::Connected;
                    info!("Voice connected");
                }
                VoiceEvent::Disconnected => {
                    *self.voice_state.write() = VoiceConnectionState::Disconnected;
                    *self.voice_channel_id.write() = None;
                    info!("Voice disconnected");
                }
                VoiceEvent::Speaking { user_id, speaking } => {
                    // Update voice state for user
                    // This would be handled in the voice window
                }
                VoiceEvent::AudioReceived { ssrc, data } => {
                    // Handle incoming audio (played by audio system)
                }
                VoiceEvent::UserJoined { user_id } => {
                    info!("User joined voice: {}", user_id);
                }
                VoiceEvent::UserLeft { user_id } => {
                    info!("User left voice: {}", user_id);
                }
                VoiceEvent::Error(e) => {
                    error!("Voice error: {}", e);
                }
            }
        }
    }

    /// Process gateway events
    fn process_gateway_events(&mut self) {
        if let Some(gateway) = &mut self.gateway {
            while let Ok(event) = gateway.try_recv_event() {
                self.handle_gateway_event(event);
            }
        }
    }

    /// Handle a gateway event
    fn handle_gateway_event(&mut self, event: GatewayEvent) {
        use GatewayEvent::*;

        match event {
            Ready(data) => {
                // Update guilds, users, etc.
                for guild in data.guilds {
                    self.guilds.write().insert(guild.id, guild);
                }
                for user in data.users {
                    self.users.write().insert(user.id, user);
                }
                for channel in data.private_channels {
                    self.channels.write().insert(channel.id, channel);
                }
                info!("Gateway READY: {} guilds", data.guilds.len());
            }
            GuildCreate(guild) => {
                self.guilds.write().insert(guild.id, guild);
            }
            GuildUpdate(guild) => {
                self.guilds.write().insert(guild.id, guild);
            }
            GuildDelete(unavailable) => {
                if unavailable.unavailable {
                    // Guild unavailable
                } else {
                    self.guilds.write().remove(&unavailable.id);
                }
            }
            ChannelCreate(channel) => {
                self.channels.write().insert(channel.id, channel);
            }
            ChannelUpdate(channel) => {
                self.channels.write().insert(channel.id, channel);
            }
            ChannelDelete(channel) => {
                self.channels.write().remove(&channel.id);
                self.messages.write().remove(&channel.id);
            }
            ChannelPinsUpdate(pins) => {
                // Handle pinned messages update
            }
            MessageCreate(msg) => {
                self.messages.write()
                    .entry(msg.channel_id)
                    .or_default()
                    .push(msg);
            }
            MessageUpdate(msg) => {
                if let Some(messages) = self.messages.write().get_mut(&msg.channel_id) {
                    if let Some(idx) = messages.iter().position(|m| m.id == msg.id) {
                        messages[idx] = msg;
                    }
                }
            }
            MessageDelete(delete) => {
                if let Some(messages) = self.messages.write().get_mut(&delete.channel_id) {
                    messages.retain(|m| m.id != delete.id);
                }
            }
            MessageDeleteBulk(bulk) => {
                if let Some(messages) = self.messages.write().get_mut(&bulk.channel_id) {
                    messages.retain(|m| !bulk.ids.contains(&m.id));
                }
            }
            VoiceStateUpdate(state) => {
                self.voice_states.write()
                    .entry(state.guild_id)
                    .or_default()
                    .insert(state.user_id, state);
            }
            VoiceServerUpdate(update) => {
                // Handle voice server update for connection
                if let Some(voice) = &self.voice {
                    // This would update the voice connection with new server info
                }
            }
            PresenceUpdate(presence) => {
                // Update user presence
            }
            UserUpdate(user) => {
                self.users.write().insert(user.id, user);
            }
            TypingStart(typing) => {
                // Show typing indicator
            }
            _ => {}
        }
    }

    /// Handle input from DualSense/keyboard
    fn handle_input(&mut self) {
        if let Some(dualsense) = &mut self.dualsense {
            while let Some(event) = dualsense.poll_event() {
                self.last_input = Instant::now();
                self.handle_input_event(event);
            }
        }
    }

    /// Handle a single input event
    fn handle_input_event(&mut self, event: InputEvent) {
        use ps5_platform::input::InputEvent::*;

        match event {
            ButtonDown(btn) => {
                // Handle button presses for navigation
                match btn {
                    ps5_platform::input::Button::Options => {
                        self.show_settings = !self.show_settings;
                    }
                    ps5_platform::input::Button::PS => {
                        self.show_user_menu = !self.show_user_menu;
                    }
                    _ => {}
                }
            }
            AxisMotion(axis, value) => {
                // Handle stick motion for scrolling/navigation
            }
            TouchpadDown(pos) | TouchpadMove(pos) => {
                // Handle touchpad as mouse
            }
            _ => {}
        }
    }

    /// Render the main UI
    fn render_ui(&mut self, ui: &mut Ui) -> Result<()> {
        // Main menu bar
        self.render_menu_bar(ui)?;

        // Main layout: Servers | Channels | Messages | Members/Voice
        let main_viewport = ui.main_viewport();
        let content_size = main_viewport.work_size;

        // Left sidebar: Servers (72px wide)
        let server_width = 72.0;
        Window::new("##ServersSidebar")
            .position([0.0, 20.0], imgui::Condition::Always)
            .size([server_width, content_size[1] - 20.0], imgui::Condition::Always)
            .flags(imgui::WindowFlags::NO_TITLE_BAR
                | imgui::WindowFlags::NO_RESIZE
                | imgui::WindowFlags::NO_MOVE
                | imgui::WindowFlags::NO_SCROLLBAR
                | imgui::WindowFlags::NO_BACKGROUND)
            .build(ui, || {
                self.servers_window.render(ui, &self.guilds.read(), &self.selected.read());
            });

        // Channel list (240px wide)
        let channel_x = server_width;
        let channel_width = 240.0;
        Window::new("##ChannelsSidebar")
            .position([channel_x, 20.0], imgui::Condition::Always)
            .size([channel_width, content_size[1] - 20.0 - 100.0], imgui::Condition::Always)
            .flags(imgui::WindowFlags::NO_TITLE_BAR
                | imgui::WindowFlags::NO_RESIZE
                | imgui::WindowFlags::NO_MOVE
                | imgui::WindowFlags::NO_BACKGROUND)
            .build(ui, || {
                self.channels_window.render(
                    ui,
                    &self.channels.read(),
                    &self.guilds.read(),
                    &self.selected.read()
                );
            });

        // Voice controls at bottom of channel list
        let voice_controls_height = 100.0;
        Window::new("##VoiceControls")
            .position([channel_x, content_size[1] - voice_controls_height], imgui::Condition::Always)
            .size([channel_width, voice_controls_height], imgui::Condition::Always)
            .flags(imgui::WindowFlags::NO_TITLE_BAR
                | imgui::WindowFlags::NO_RESIZE
                | imgui::WindowFlags::NO_MOVE)
            .build(ui, || {
                self.voice_controls.render(
                    ui,
                    &self.voice_state.read(),
                    &self.voice_channel_id.read(),
                    self.voice.as_ref()
                );
            });

        // Main content area: Messages
        let messages_x = server_width + channel_width;
        let messages_width = content_size[0] - messages_x - 240.0; // Leave room for member list
        Window::new("##Messages")
            .position([messages_x, 20.0], imgui::Condition::Always)
            .size([messages_width, content_size[1] - 20.0 - 100.0], imgui::Condition::Always)
            .flags(imgui::WindowFlags::NO_TITLE_BAR
                | imgui::WindowFlags::NO_RESIZE
                | imgui::WindowFlags::NO_MOVE)
            .build(ui, || {
                self.messages_window.render(
                    ui,
                    &self.messages.read(),
                    &self.channels.read(),
                    &self.users.read(),
                    &self.selected.read(),
                    &self.current_user.read()
                );
            });

        // Message input area
        Window::new("##MessageInput")
            .position([messages_x, content_size[1] - 100.0], imgui::Condition::Always)
            .size([messages_width, 100.0], imgui::Condition::Always)
            .flags(imgui::WindowFlags::NO_TITLE_BAR
                | imgui::WindowFlags::NO_RESIZE
                | imgui::WindowFlags::NO_MOVE)
            .build(ui, || {
                self.render_message_input(ui)?;
                Ok(())
            });

        // Right sidebar: Member list (240px wide)
        let members_x = content_size[0] - 240.0;
        Window::new("##MembersSidebar")
            .position([members_x, 20.0], imgui::Condition::Always)
            .size([240.0, content_size[1] - 20.0], imgui::Condition::Always)
            .flags(imgui::WindowFlags::NO_TITLE_BAR
                | imgui::WindowFlags::NO_RESIZE
                | imgui::WindowFlags::NO_MOVE)
            .build(ui, || {
                self.member_list.render(
                    ui,
                    &self.voice_states.read(),
                    &self.guilds.read(),
                    &self.users.read(),
                    &self.selected.read()
                );
            });

        // Popout windows
        if self.show_settings {
            self.settings_window.render(
                ui,
                &mut self.show_settings,
                &mut self.settings_tab,
                &self.current_user.read()
            );
        }

        if self.show_user_menu {
            self.user_menu_window.render(
                ui,
                &mut self.show_user_menu,
                &self.current_user.read()
            );
        }

        if self.show_voice_popout {
            self.voice_window.render(
                ui,
                &mut self.show_voice_popout,
                &self.voice_state.read(),
                self.voice.as_ref()
            );
        }

        // Debug overlay (FPS, etc.)
        #[cfg(debug_assertions)]
        self.render_debug_overlay(ui);

        Ok(())
    }

    /// Render top menu bar
    fn render_menu_bar(&mut self, ui: &Ui) -> Result<()> {
        Window::new("##MenuBar")
            .position([0.0, 0.0], imgui::Condition::Always)
            .size([ui.main_viewport().work_size[0], 20.0], imgui::Condition::Always)
            .flags(imgui::WindowFlags::NO_TITLE_BAR
                | imgui::WindowFlags::NO_RESIZE
                | imgui::WindowFlags::NO_MOVE
                | imgui::WindowFlags::NO_SCROLLBAR
                | imgui::WindowFlags::MENU_BAR)
            .build(ui, || {
                if ui.begin_menu_bar() {
                    // App menu
                    if ui.begin_menu("Discord") {
                        if ui.menu_item("Settings") {
                            self.show_settings = true;
                        }
                        if ui.menu_item("Quit") {
                            *self.state.write() = AppState::Shutdown;
                        }
                        ui.end_menu();
                    }

                    // Server menu (if guild selected)
                    if let SelectedItem::Server(guild_id) = *self.selected.read() {
                        if let Some(guild) = self.guilds.read().get(&guild_id) {
                            if ui.begin_menu(&format!("{}", guild.name)) {
                                if ui.menu_item("Leave Server") {
                                    // TODO: Leave server
                                }
                                ui.end_menu();
                            }
                        }
                    }

                    // Channel menu (if channel selected)
                    if let SelectedItem::Channel(channel_id) = *self.selected.read() {
                        if let Some(channel) = self.channels.read().get(&channel_id) {
                            if ui.begin_menu(&format!("#{}", channel.name.unwrap_or_default())) {
                                if ui.menu_item("Mute Channel") {
                                    // TODO: Mute channel
                                }
                                if ui.menu_item("Notification Settings") {
                                    // TODO: Notification settings
                                }
                                ui.end_menu();
                            }
                        }
                    }

                    ui.end_menu_bar();
                }
            });

        Ok(())
    }

    /// Render message input area
    fn render_message_input(&mut self, ui: &mut Ui) -> Result<()> {
        let channel_id = match *self.selected.read() {
            SelectedItem::Channel(id) => Some(id),
            SelectedItem::DMChannel(id) => Some(id),
            _ => None,
        };

        if let Some(channel_id) = channel_id {
            let mut buffer = String::new();
            let input_flags = imgui::InputTextFlags::ENTER_RETURNS_TRUE
                | imgui::InputTextFlags::CTRL_ENTER_FOR_NEW_LINE;

            let available_width = ui.content_region_avail()[0] - 80.0;
            ui.push_item_width(available_width);

            if ui.input_text("##MessageInput", &mut buffer, input_flags) {
                if !buffer.trim().is_empty() {
                    self.send_message(channel_id, &buffer);
                    buffer.clear();
                }
            }
            ui.pop_item_width();

            ui.same_line();
            if ui.button("Send", [72.0, 0.0]) {
                // Get text from input
                // Note: In real implementation, we'd store the buffer
            }
        } else {
            ui.text_colored(DISCORD_TEXT_MUTED, "Select a channel to send messages");
        }

        Ok(())
    }

    /// Send a message
    fn send_message(&self, channel_id: ChannelId, content: &str) {
        if let Some(rest) = &self.rest {
            let content = content.to_string();
            let rest = rest.clone();
            tokio::spawn(async move {
                if let Err(e) = rest.create_message(channel_id, &content, None).await {
                    error!("Failed to send message: {}", e);
                }
            });
        }
    }

    /// Render debug overlay
    #[cfg(debug_assertions)]
    fn render_debug_overlay(&mut self, ui: &Ui) {
        Window::new("Debug")
            .flags(imgui::WindowFlags::NO_TITLE_BAR
                | imgui::WindowFlags::NO_RESIZE
                | imgui::WindowFlags::ALWAYS_AUTO_RESIZE
                | imgui::WindowFlags::NO_BACKGROUND)
            .position([10.0, 30.0], imgui::Condition::FirstUseEver)
            .build(ui, || {
                ui.text(format!("FPS: {:.1}", self.current_fps));
                ui.text(format!("Frame time: {:.2}ms",
                    self.frame_times.last().unwrap_or(&0.0) * 1000.0));
                ui.text(format!("Guilds: {}", self.guilds.read().len()));
                ui.text(format!("Channels: {}", self.channels.read().len()));
                ui.text(format!("Messages: {}", self.messages.read().values().map(|v| v.len()).sum::<usize>()));
                ui.text(format!("Voice: {:?}", *self.voice_state.read()));
                if let Some(voice) = &self.voice {
                    ui.text(format!("Voice connected: {}", voice.is_connected()));
                }
            });
    }

    /// Join a voice channel
    pub async fn join_voice_channel(&mut self, channel_id: ChannelId) -> Result<()> {
        info!("Joining voice channel: {}", channel_id);

        *self.voice_channel_id.write() = Some(channel_id);
        *self.voice_state.write() = VoiceConnectionState::Connecting;

        if let Some(voice) = &self.voice {
            voice.connect().await?;
        }

        Ok(())
    }

    /// Leave voice channel
    pub async fn leave_voice_channel(&mut self) -> Result<()> {
        info!("Leaving voice channel");

        if let Some(voice) = &self.voice {
            voice.disconnect().await?;
        }

        *self.voice_channel_id.write() = None;
        *self.voice_state.write() = VoiceConnectionState::Disconnected;

        Ok(())
    }

    /// Toggle mute
    pub async fn toggle_mute(&mut self) -> Result<()> {
        if let Some(voice) = &self.voice {
            // This would toggle local mute state
            // voice.set_speaking(false).await?;
        }
        Ok(())
    }

    /// Toggle deafen
    pub async fn toggle_deafen(&mut self) -> Result<()> {
        // Toggle deafen state
        Ok(())
    }

    /// Shutdown the application
    pub async fn shutdown(&mut self) -> Result<()> {
        info!("Shutting down application...");
        *self.state.write() = AppState::Shutdown;

        // Leave voice channel
        self.leave_voice_channel().await?;

        // Disconnect gateway
        if let Some(gateway) = &mut self.gateway {
            gateway.disconnect().await?;
        }

        Ok(())
    }

    /// Get current application state
    pub fn state(&self) -> AppState {
        *self.state.read()
    }

    /// Get selected item
    pub fn selected(&self) -> SelectedItem {
        self.selected.read().clone()
    }

    /// Set selected item
    pub fn set_selected(&self, item: SelectedItem) {
        *self.selected.write() = item;
    }
}

impl Default for App {
    fn default() -> Self {
        Self::new().expect("Failed to create App")
    }
}