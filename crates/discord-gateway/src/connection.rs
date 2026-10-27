//! Gateway WebSocket Connection Management

use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use anyhow::{Context, Result};
use futures::{SinkExt, StreamExt};
use parking_lot::RwLock;
use tokio::net::TcpStream;
use tokio::sync::mpsc;
use tokio::time::{interval, timeout};
use tokio_tungstenite::{connect_async, MaybeTlsStream, WebSocketStream, tungstenite::protocol::Message};
use tracing::{debug, error, info, warn};

use crate::{DiscordError, Result as GatewayResult};
use discord_common::discord_types::*;

/// Gateway connection state
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ConnectionState {
    Disconnected,
    Connecting,
    Connected,
    Resuming,
    Identifying,
    Ready,
    Closing,
}

/// Gateway connection configuration
#[derive(Debug, Clone)]
pub struct GatewayConfig {
    pub url: String,
    pub token: String,
    pub intents: u64,
    pub shard: Option<[u32; 2]>,
    pub presence: Option<PresenceData>,
    pub heartbeat_interval: u64,
    pub reconnect_attempts: u32,
    pub reconnect_delay: Duration,
}

impl Default for GatewayConfig {
    fn default() -> Self {
        Self {
            url: "wss://gateway.discord.gg".to_string(),
            token: String::new(),
            intents: Intents::all().bits(),
            shard: None,
            presence: None,
            heartbeat_interval: 41250,
            reconnect_attempts: 5,
            reconnect_delay: Duration::from_secs(5),
        }
    }
}

/// Gateway event sender type
pub type EventSender = mpsc::UnboundedSender<GatewayEvent>;
/// Gateway event receiver type
pub type EventReceiver = mpsc::UnboundedReceiver<GatewayEvent>;

/// Gateway events that can be dispatched
#[derive(Debug, Clone)]
pub enum GatewayEvent {
    Ready(ReadyData),
    Resumed,
    MessageCreate(Message),
    MessageUpdate(Message),
    MessageDelete(MessageDelete),
    MessageDeleteBulk(MessageDeleteBulk),
    ChannelCreate(Channel),
    ChannelUpdate(Channel),
    ChannelDelete(Channel),
    ChannelPinsUpdate(ChannelPinsUpdate),
    GuildCreate(Guild),
    GuildUpdate(Guild),
    GuildDelete(UnavailableGuild),
    GuildMemberAdd(GuildMember),
    GuildMemberUpdate(GuildMemberUpdate),
    GuildMemberRemove(GuildMemberRemove),
    GuildMembersChunk(GuildMembersChunk),
    GuildRoleCreate(Role),
    GuildRoleUpdate(Role),
    GuildRoleDelete(RoleDelete),
    GuildBanAdd(BanAdd),
    GuildBanRemove(BanRemove),
    GuildEmojisUpdate(GuildEmojisUpdate),
    GuildStickersUpdate(GuildStickersUpdate),
    GuildIntegrationsUpdate(GuildIntegrationsUpdate),
    GuildWebhooksUpdate(GuildWebhooksUpdate),
    InviteCreate(Invite),
    InviteDelete(InviteDelete),
    ThreadCreate(Channel),
    ThreadUpdate(Channel),
    ThreadDelete(Channel),
    ThreadListSync(ThreadListSync),
    ThreadMemberUpdate(ThreadMemberUpdate),
    ThreadMembersUpdate(ThreadMembersUpdate),
    VoiceStateUpdate(VoiceState),
    VoiceServerUpdate(VoiceServerUpdate),
    PresenceUpdate(PresenceUpdate),
    UserUpdate(User),
    InteractionCreate(Interaction),
    ApplicationCommandCreate(ApplicationCommand),
    ApplicationCommandUpdate(ApplicationCommand),
    ApplicationCommandDelete(ApplicationCommandDelete),
    Raw(RawGatewayEvent),
}

/// Raw gateway event for unhandled types
#[derive(Debug, Clone)]
pub struct RawGatewayEvent {
    pub event_type: String,
    pub data: serde_json::Value,
    pub sequence: Option<u64>,
}

/// Gateway connection manager
pub struct GatewayConnection {
    config: GatewayConfig,
    state: Arc<RwLock<ConnectionState>>,
    ws_stream: Arc<Mutex<Option<WebSocketStream<MaybeTlsStream<TcpStream>>>>>,
    sequence: Arc<RwLock<Option<u64>>>,
    session_id: Arc<RwLock<Option<String>>>,
    heartbeat_interval: Arc<RwLock<u64>>,
    event_tx: EventSender,
    last_heartbeat_ack: Arc<RwLock<Option<Instant>>>,
    heartbeat_task: Arc<Mutex<Option<tokio::task::JoinHandle<()>>>>,
    reconnect_task: Arc<Mutex<Option<tokio::task::JoinHandle<()>>>>,
    running: Arc<RwLock<bool>>,
}

impl GatewayConnection {
    /// Create a new gateway connection
    pub fn new(config: GatewayConfig) -> (Self, EventReceiver) {
        let (tx, rx) = mpsc::unbounded_channel();

        let conn = Self {
            config,
            state: Arc::new(RwLock::new(ConnectionState::Disconnected)),
            ws_stream: Arc::new(Mutex::new(None)),
            sequence: Arc::new(RwLock::new(None)),
            session_id: Arc::new(RwLock::new(None)),
            heartbeat_interval: Arc::new(RwLock::new(41250)),
            event_tx: tx,
            last_heartbeat_ack: Arc::new(RwLock::new(None)),
            heartbeat_task: Arc::new(Mutex::new(None)),
            reconnect_task: Arc::new(Mutex::new(None)),
            running: Arc::new(RwLock::new(false)),
        };

        (conn, rx)
    }

    /// Connect to the gateway
    pub async fn connect(&self) -> GatewayResult<()> {
        info!("Connecting to Discord Gateway...");
        *self.state.write() = ConnectionState::Connecting;
        *self.running.write() = true;

        let url = format!("{}?v=10&encoding=json", self.config.url);

        let (ws_stream, _) = connect_async(&url)
            .await
            .context("Failed to connect to gateway")?;

        info!("WebSocket connected");
        *self.ws_stream.lock().unwrap() = Some(ws_stream);
        *self.state.write() = ConnectionState::Connected;

        // Start reading messages
        self.start_read_loop().await;

        Ok(())
    }

    /// Start the read loop
    async fn start_read_loop(&self) {
        let ws_stream = self.ws_stream.clone();
        let sequence = self.sequence.clone();
        let session_id = self.session_id.clone();
        let heartbeat_interval = self.heartbeat_interval.clone();
        let event_tx = self.event_tx.clone();
        let state = self.state.clone();
        let last_heartbeat_ack = self.last_heartbeat_ack.clone();
        let running = self.running.clone();

        tokio::spawn(async move {
            let mut ws_lock = ws_stream.lock().unwrap();
            if let Some(mut ws) = ws_lock.take() {
                while *running.read() {
                    match ws.next().await {
                        Some(Ok(msg)) => {
                            if let Err(e) = Self::handle_message(
                                msg,
                                &sequence,
                                &session_id,
                                &heartbeat_interval,
                                &event_tx,
                                &state,
                                &last_heartbeat_ack,
                            ).await {
                                error!("Error handling message: {}", e);
                            }
                        }
                        Some(Err(e)) => {
                            error!("WebSocket error: {}", e);
                            break;
                        }
                        None => {
                            info!("WebSocket closed");
                            break;
                        }
                    }
                }
                *ws_lock = Some(ws);
            }
        });
    }

    /// Handle incoming gateway message
    async fn handle_message(
        msg: Message,
        sequence: &Arc<RwLock<Option<u64>>>,
        session_id: &Arc<RwLock<Option<String>>>,
        heartbeat_interval: &Arc<RwLock<u64>>,
        event_tx: &EventSender,
        state: &Arc<RwLock<ConnectionState>>,
        last_heartbeat_ack: &Arc<RwLock<Option<Instant>>>,
    ) -> GatewayResult<()> {
        match msg {
            Message::Text(text) => {
                let payload: GatewayPayload = serde_json::from_str(&text)
                    .context("Failed to parse gateway payload")?;

                // Update sequence
                if let Some(s) = payload.s {
                    *sequence.write() = Some(s);
                }

                match payload.op {
                    OpCode::Dispatch => {
                        Self::handle_dispatch(payload, event_tx).await?;
                    }
                    OpCode::Hello => {
                        let hello: HelloData = serde_json::from_value(payload.d)?;
                        *heartbeat_interval.write() = hello.heartbeat_interval;
                        info!("Received HELLO, heartbeat interval: {}ms", hello.heartbeat_interval);

                        // Send Identify or Resume
                        if session_id.read().is_some() && sequence.read().is_some() {
                            *state.write() = ConnectionState::Resuming;
                            Self::send_resume(&mut ws_stream.lock().unwrap(), session_id, sequence).await?;
                        } else {
                            *state.write() = ConnectionState::Identifying;
                            Self::send_identify(&mut ws_stream.lock().unwrap(), &config).await?;
                        }
                    }
                    OpCode::HeartbeatAck => {
                        *last_heartbeat_ack.write() = Some(Instant::now());
                        debug!("Received heartbeat ACK");
                    }
                    OpCode::Reconnect => {
                        warn!("Received RECONNECT opcode");
                        // Trigger reconnect
                    }
                    OpCode::InvalidSession => {
                        let resumable: bool = serde_json::from_value(payload.d).unwrap_or(false);
                        warn!("Invalid session, resumable: {}", resumable);
                        if !resumable {
                            *session_id.write() = None;
                            *sequence.write() = None;
                        }
                    }
                    _ => {
                        debug!("Unhandled opcode: {:?}", payload.op);
                    }
                }
            }
            Message::Binary(_) => {
                // Discord doesn't send binary by default
            }
            Message::Ping(data) => {
                // Respond with pong
                if let Some(ws) = ws_stream.lock().unwrap().as_mut() {
                    ws.send(Message::Pong(data)).await.ok();
                }
            }
            Message::Pong(_) => {}
            Message::Close(frame) => {
                info!("Gateway closed: {:?}", frame);
                *state.write() = ConnectionState::Closing;
            }
            _ => {}
        }
        Ok(())
    }

    /// Handle DISPATCH events
    async fn handle_dispatch(
        payload: GatewayPayload,
        event_tx: &EventSender,
    ) -> GatewayResult<()> {
        let event_type = payload.t.unwrap_or_default();
        let data = payload.d;

        let event = match event_type.as_str() {
            "READY" => {
                let ready: ReadyData = serde_json::from_value(data)?;
                *session_id.write() = Some(ready.session_id.clone());
                *state.write() = ConnectionState::Ready;
                GatewayEvent::Ready(ready)
            }
            "RESUMED" => {
                info!("Session resumed");
                GatewayEvent::Resumed
            }
            "MESSAGE_CREATE" => {
                let msg: Message = serde_json::from_value(data)?;
                GatewayEvent::MessageCreate(msg)
            }
            "MESSAGE_UPDATE" => {
                let msg: Message = serde_json::from_value(data)?;
                GatewayEvent::MessageUpdate(msg)
            }
            "MESSAGE_DELETE" => {
                let msg: MessageDelete = serde_json::from_value(data)?;
                GatewayEvent::MessageDelete(msg)
            }
            "MESSAGE_DELETE_BULK" => {
                let msg: MessageDeleteBulk = serde_json::from_value(data)?;
                GatewayEvent::MessageDeleteBulk(msg)
            }
            "CHANNEL_CREATE" => {
                let ch: Channel = serde_json::from_value(data)?;
                GatewayEvent::ChannelCreate(ch)
            }
            "CHANNEL_UPDATE" => {
                let ch: Channel = serde_json::from_value(data)?;
                GatewayEvent::ChannelUpdate(ch)
            }
            "CHANNEL_DELETE" => {
                let ch: Channel = serde_json::from_value(data)?;
                GatewayEvent::ChannelDelete(ch)
            }
            "CHANNEL_PINS_UPDATE" => {
                let ch: ChannelPinsUpdate = serde_json::from_value(data)?;
                GatewayEvent::ChannelPinsUpdate(ch)
            }
            "GUILD_CREATE" => {
                let guild: Guild = serde_json::from_value(data)?;
                GatewayEvent::GuildCreate(guild)
            }
            "GUILD_UPDATE" => {
                let guild: Guild = serde_json::from_value(data)?;
                GatewayEvent::GuildUpdate(guild)
            }
            "GUILD_DELETE" => {
                let guild: UnavailableGuild = serde_json::from_value(data)?;
                GatewayEvent::GuildDelete(guild)
            }
            "GUILD_MEMBER_ADD" => {
                let member: GuildMember = serde_json::from_value(data)?;
                GatewayEvent::GuildMemberAdd(member)
            }
            "GUILD_MEMBER_UPDATE" => {
                let member: GuildMemberUpdate = serde_json::from_value(data)?;
                GatewayEvent::GuildMemberUpdate(member)
            }
            "GUILD_MEMBER_REMOVE" => {
                let member: GuildMemberRemove = serde_json::from_value(data)?;
                GatewayEvent::GuildMemberRemove(member)
            }
            "GUILD_MEMBERS_CHUNK" => {
                let chunk: GuildMembersChunk = serde_json::from_value(data)?;
                GatewayEvent::GuildMembersChunk(chunk)
            }
            "GUILD_ROLE_CREATE" => {
                let role: Role = serde_json::from_value(data)?;
                GatewayEvent::GuildRoleCreate(role)
            }
            "GUILD_ROLE_UPDATE" => {
                let role: Role = serde_json::from_value(data)?;
                GatewayEvent::GuildRoleUpdate(role)
            }
            "GUILD_ROLE_DELETE" => {
                let role: RoleDelete = serde_json::from_value(data)?;
                GatewayEvent::GuildRoleDelete(role)
            }
            "VOICE_STATE_UPDATE" => {
                let vs: VoiceState = serde_json::from_value(data)?;
                GatewayEvent::VoiceStateUpdate(vs)
            }
            "VOICE_SERVER_UPDATE" => {
                let vs: VoiceServerUpdate = serde_json::from_value(data)?;
                GatewayEvent::VoiceServerUpdate(vs)
            }
            "PRESENCE_UPDATE" => {
                let pres: PresenceUpdate = serde_json::from_value(data)?;
                GatewayEvent::PresenceUpdate(pres)
            }
            "USER_UPDATE" => {
                let user: User = serde_json::from_value(data)?;
                GatewayEvent::UserUpdate(user)
            }
            _ => {
                debug!("Unhandled event type: {}", event_type);
                GatewayEvent::Raw(RawGatewayEvent {
                    event_type,
                    data,
                    sequence: payload.s,
                })
            }
        };

        event_tx.send(event).ok();
        Ok(())
    }

    /// Send IDENTIFY payload
    async fn send_identify(
        ws: &mut WebSocketStream<MaybeTlsStream<TcpStream>>,
        config: &GatewayConfig,
    ) -> GatewayResult<()> {
        let properties = IdentifyProperties {
            os: "FreeBSD".to_string(),
            browser: "DiscordPS5".to_string(),
            device: "DiscordPS5".to_string(),
        };

        let identify = IdentifyData {
            token: config.token.clone(),
            properties,
            compress: Some(false),
            large_threshold: Some(250),
            shard: config.shard,
            presence: config.presence.clone(),
            intents: config.intents,
        };

        let payload = GatewayPayload {
            op: OpCode::Identify,
            d: serde_json::to_value(identify)?,
            s: None,
            t: None,
        };

        let msg = Message::Text(serde_json::to_string(&payload)?);
        ws.send(msg).await?;
        info!("Sent IDENTIFY");
        Ok(())
    }

    /// Send RESUME payload
    async fn send_resume(
        ws: &mut WebSocketStream<MaybeTlsStream<TcpStream>>,
        session_id: &Arc<RwLock<Option<String>>>,
        sequence: &Arc<RwLock<Option<u64>>>,
    ) -> GatewayResult<()> {
        let resume = ResumeData {
            token: config.token.clone(),
            session_id: session_id.read().clone().unwrap(),
            seq: sequence.read().unwrap(),
        };

        let payload = GatewayPayload {
            op: OpCode::Resume,
            d: serde_json::to_value(resume)?,
            s: None,
            t: None,
        };

        let msg = Message::Text(serde_json::to_string(&payload)?);
        ws.send(msg).await?;
        info!("Sent RESUME");
        Ok(())
    }

    /// Send heartbeat
    pub async fn send_heartbeat(&self) -> GatewayResult<()> {
        let seq = *self.sequence.read();
        let payload = GatewayPayload {
            op: OpCode::Heartbeat,
            d: serde_json::to_value(seq)?,
            s: None,
            t: None,
        };

        let msg = Message::Text(serde_json::to_string(&payload)?);
        if let Some(ws) = self.ws_stream.lock().unwrap().as_mut() {
            ws.send(msg).await?;
        }
        Ok(())
    }

    /// Get current connection state
    pub fn state(&self) -> ConnectionState {
        *self.state.read()
    }

    /// Check if connected and ready
    pub fn is_ready(&self) -> bool {
        matches!(*self.state.read(), ConnectionState::Ready)
    }

    /// Disconnect gracefully
    pub async fn disconnect(&self) -> GatewayResult<()> {
        info!("Disconnecting from gateway...");
        *self.running.write() = false;
        *self.state.write() = ConnectionState::Closing;

        // Stop heartbeat task
        if let Some(handle) = self.heartbeat_task.lock().unwrap().take() {
            handle.abort();
        }

        // Stop reconnect task
        if let Some(handle) = self.reconnect_task.lock().unwrap().take() {
            handle.abort();
        }

        // Close WebSocket
        if let Some(mut ws) = self.ws_stream.lock().unwrap().take() {
            let _ = ws.close(None).await;
        }

        info!("Disconnected from gateway");
        Ok(())
    }
}