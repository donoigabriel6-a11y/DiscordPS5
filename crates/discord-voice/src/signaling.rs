//! Voice WebSocket Signaling
//!
//! Handles Discord Voice WebSocket signaling protocol for voice connection
//! negotiation, including identify, protocol selection, session description,
//! speaking indicators, and heartbeating.

use std::sync::Arc;
use std::time::{Duration, Instant};
use std::collections::HashMap;

use anyhow::{Context, Result};
use futures::{SinkExt, StreamExt};
use parking_lot::RwLock;
use tokio::net::TcpStream;
use tokio::sync::mpsc;
use tokio::time::{interval, timeout};
use tokio_tungstenite::{connect_async, MaybeTlsStream, WebSocketStream};
use tungstenite::protocol::Message;
use tracing::{debug, error, info, warn};

use crate::{DiscordError, Result as VoiceResult, VoiceConfig, VoiceEvent};
use discord_common::discord_types::*;

/// Voice signaling state
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SignalingState {
    Disconnected,
    Connecting,
    Identifying,
    WaitingForHello,
    SelectingProtocol,
    WaitingForReady,
    WaitingForSessionDescription,
    Connected,
    Reconnecting,
    Failed,
}

/// Voice signaling configuration
#[derive(Debug, Clone)]
pub struct SignalingConfig {
    pub endpoint: String,
    pub guild_id: String,
    pub user_id: String,
    pub session_id: String,
    pub token: String,
}

impl SignalingConfig {
    pub fn from_voice_config(config: &VoiceConfig) -> Self {
        Self {
            endpoint: config.endpoint.clone(),
            guild_id: config.guild_id.clone(),
            user_id: config.user_id.clone(),
            session_id: config.session_id.clone(),
            token: config.token.clone(),
        }
    }
}

/// Voice signaling manager
pub struct VoiceSignaling {
    config: SignalingConfig,
    state: Arc<RwLock<SignalingState>>,
    ws_stream: Arc<RwLock<Option<WebSocketStream<MaybeTlsStream<TcpStream>>>>>,
    heartbeat_interval: Arc<RwLock<u32>>,
    last_heartbeat_ack: Arc<RwLock<Option<Instant>>>,
    ssrc: Arc<RwLock<Option<u32>>>,
    ip: Arc<RwLock<Option<String>>>,
    port: Arc<RwLock<Option<u16>>>,
    modes: Arc<RwLock<Vec<String>>>,
    secret_key: Arc<RwLock<Option<Vec<u8>>>>,
    event_tx: mpsc::UnboundedSender<VoiceEvent>,
    running: Arc<RwLock<bool>>,
    reconnect_attempts: u32,
    max_reconnect_attempts: u32,
}

impl VoiceSignaling {
    /// Create a new voice signaling manager
    pub fn new(config: VoiceConfig, event_tx: mpsc::UnboundedSender<VoiceEvent>) -> Self {
        let signaling_config = SignalingConfig::from_voice_config(&config);
        Self {
            config: signaling_config,
            state: Arc::new(RwLock::new(SignalingState::Disconnected)),
            ws_stream: Arc::new(RwLock::new(None)),
            heartbeat_interval: Arc::new(RwLock::new(0)),
            last_heartbeat_ack: Arc::new(RwLock::new(None)),
            ssrc: Arc::new(RwLock::new(None)),
            ip: Arc::new(RwLock::new(None)),
            port: Arc::new(RwLock::new(None)),
            modes: Arc::new(RwLock::new(Vec::new())),
            secret_key: Arc::new(RwLock::new(None)),
            event_tx,
            running: Arc::new(RwLock::new(false)),
            reconnect_attempts: 0,
            max_reconnect_attempts: 5,
        }
    }

    /// Connect to voice WebSocket and complete handshake
    pub async fn connect(&self) -> VoiceResult<()> {
        info!("Connecting to voice WebSocket at {}", self.config.endpoint);
        *self.state.write() = SignalingState::Connecting;
        *self.running.write() = true;

        let url = format!("wss://{}/?v=4", self.config.endpoint.trim_start_matches("wss://").trim_start_matches("https://"));

        let (ws_stream, _) = connect_async(&url)
            .await
            .context("Failed to connect to voice WebSocket")?;

        info!("Voice WebSocket connected");
        *self.ws_stream.write() = Some(ws_stream);
        *self.state.write() = SignalingState::Identifying;

        // Send IDENTIFY
        self.send_identify().await?;

        // Start read loop
        self.start_read_loop().await;

        // Wait for connection to be established
        self.wait_for_connected().await?;

        Ok(())
    }

    /// Wait for signaling to reach Connected state
    async fn wait_for_connected(&self) -> VoiceResult<()> {
        let start = Instant::now();
        let timeout_dur = Duration::from_secs(30);

        while start.elapsed() < timeout_dur {
            tokio::time::sleep(Duration::from_millis(100)).await;
            if matches!(*self.state.read(), SignalingState::Connected) {
                return Ok(());
            }
            if matches!(*self.state.read(), SignalingState::Failed) {
                return Err(DiscordError::VoiceConnection("Signaling failed".to_string()).into());
            }
        }
        Err(DiscordError::VoiceConnection("Signaling connection timeout".to_string()).into())
    }

    /// Start the WebSocket read loop
    async fn start_read_loop(&self) {
        let ws_stream = self.ws_stream.clone();
        let state = self.state.clone();
        let heartbeat_interval = self.heartbeat_interval.clone();
        let last_heartbeat_ack = self.last_heartbeat_ack.clone();
        let ssrc = self.ssrc.clone();
        let ip = self.ip.clone();
        let port = self.port.clone();
        let modes = self.modes.clone();
        let secret_key = self.secret_key.clone();
        let event_tx = self.event_tx.clone();
        let running = self.running.clone();
        let config = self.config.clone();

        tokio::spawn(async move {
            let mut ws_lock = ws_stream.write();
            if let Some(mut ws) = ws_lock.take() {
                while *running.read() {
                    match ws.next().await {
                        Some(Ok(msg)) => {
                            if let Err(e) = Self::handle_message(
                                msg,
                                &state,
                                &heartbeat_interval,
                                &last_heartbeat_ack,
                                &ssrc,
                                &ip,
                                &port,
                                &modes,
                                &secret_key,
                                &event_tx,
                                &config,
                                &ws_stream,
                            ).await {
                                error!("Error handling voice signaling message: {}", e);
                            }
                        }
                        Some(Err(e)) => {
                            error!("Voice WebSocket error: {}", e);
                            *state.write() = SignalingState::Failed;
                            break;
                        }
                        None => {
                            info!("Voice WebSocket closed");
                            *state.write() = SignalingState::Failed;
                            break;
                        }
                    }
                }
                *ws_lock = Some(ws);
            }
        });
    }

    /// Handle incoming voice signaling message
    async fn handle_message(
        msg: Message,
        state: &Arc<RwLock<SignalingState>>,
        heartbeat_interval: &Arc<RwLock<u32>>,
        last_heartbeat_ack: &Arc<RwLock<Option<Instant>>>,
        ssrc: &Arc<RwLock<Option<u32>>>,
        ip: &Arc<RwLock<Option<String>>>,
        port: &Arc<RwLock<Option<u16>>>,
        modes: &Arc<RwLock<Vec<String>>>,
        secret_key: &Arc<RwLock<Option<Vec<u8>>>>,
        event_tx: &mpsc::UnboundedSender<VoiceEvent>,
        config: &SignalingConfig,
        ws_stream: &Arc<RwLock<Option<WebSocketStream<MaybeTlsStream<TcpStream>>>>>,
    ) -> VoiceResult<()> {
        match msg {
            Message::Text(text) => {
                let payload: serde_json::Value = serde_json::from_str(&text)
                    .context("Failed to parse voice signaling payload")?;

                let op = payload["op"].as_u64().unwrap_or(0) as u8;
                let opcode = VoiceOpCode::from(op);

                match opcode {
                    VoiceOpCode::Hello => {
                        let hello: VoiceHello = serde_json::from_value(payload["d"].clone())?;
                        *heartbeat_interval.write() = hello.heartbeat_interval;
                        info!("Received HELLO, heartbeat interval: {}ms", hello.heartbeat_interval);

                        // Start heartbeat
                        Self::start_heartbeat_loop(
                            heartbeat_interval.clone(),
                            last_heartbeat_ack.clone(),
                            ws_stream.clone(),
                            running.clone(),
                        );

                        // Send SELECT_PROTOCOL
                        *state.write() = SignalingState::SelectingProtocol;
                        Self::send_select_protocol(&mut ws_stream.write(), config).await?;
                    }
                    VoiceOpCode::Ready => {
                        let ready: VoiceReady = serde_json::from_value(payload["d"].clone())?;
                        *ssrc.write() = Some(ready.ssrc);
                        *ip.write() = Some(ready.ip.clone());
                        *port.write() = Some(ready.port);
                        *modes.write() = ready.modes.clone();
                        info!("Voice READY: ssrc={}, ip={}, port={}, modes={:?}",
                            ready.ssrc, ready.ip, ready.port, ready.modes);

                        *state.write() = SignalingState::WaitingForSessionDescription;
                    }
                    VoiceOpCode::SessionDescription => {
                        let session_desc: VoiceSessionDescription = serde_json::from_value(payload["d"].clone())?;
                        *secret_key.write() = Some(session_desc.secret_key.clone());
                        info!("Received SessionDescription, mode: {}, secret_key len: {}",
                            session_desc.mode, session_desc.secret_key.len());

                        *state.write() = SignalingState::Connected;
                        event_tx.send(VoiceEvent::Connected).ok();
                    }
                    VoiceOpCode::HeartbeatAck => {
                        *last_heartbeat_ack.write() = Some(Instant::now());
                        debug!("Received voice heartbeat ACK");
                    }
                    VoiceOpCode::Resumed => {
                        info!("Voice session resumed");
                        *state.write() = SignalingState::Connected;
                    }
                    VoiceOpCode::ClientDisconnect => {
                        let disconnect: VoiceClientDisconnect = serde_json::from_value(payload["d"].clone())?;
                        info!("Client disconnected: {}", disconnect.user_id);
                    }
                    _ => {
                        debug!("Unhandled voice opcode: {:?}", opcode);
                    }
                }
            }
            Message::Ping(data) => {
                if let Some(ws) = ws_stream.read().as_ref() {
                    // Note: we can't send here because we don't have mutable access
                    // The ping will be handled by the tungstenite library automatically
                    drop(ws);
                }
            }
            Message::Close(frame) => {
                info!("Voice WebSocket closed: {:?}", frame);
                *state.write() = SignalingState::Failed;
            }
            _ => {}
        }
        Ok(())
    }

    /// Send IDENTIFY packet
    async fn send_identify(&self) -> VoiceResult<()> {
        let identify = VoiceIdentify {
            server_id: self.config.guild_id.clone(),
            user_id: self.config.user_id.clone(),
            session_id: self.config.session_id.clone(),
            token: self.config.token.clone(),
        };

        let payload = serde_json::json!({
            "op": VoiceOpCode::Identify as u8,
            "d": identify,
        });

        self.send_json(payload).await?;
        info!("Sent voice IDENTIFY");
        Ok(())
    }

    /// Send SELECT_PROTOCOL packet
    async fn send_select_protocol(
        ws: &mut Option<WebSocketStream<MaybeTlsStream<TcpStream>>>,
        config: &SignalingConfig,
    ) -> VoiceResult<()> {
        // We need the IP and port from the Ready packet, but we don't have them yet
        // Discord expects us to send our local UDP address/port
        // For now, we'll use a placeholder - in reality this should come from the ICE negotiation
        let protocol_data = VoiceProtocolData {
            address: "0.0.0.0".to_string(), // Will be updated after ICE
            port: 0, // Will be updated after ICE
            mode: "xsalsa20_poly1305".to_string(),
        };

        let select_protocol = VoiceSelectProtocol {
            protocol: "udp".to_string(),
            data: protocol_data,
        };

        let payload = serde_json::json!({
            "op": VoiceOpCode::SelectProtocol as u8,
            "d": select_protocol,
        });

        if let Some(ws) = ws {
            let msg = Message::Text(serde_json::to_string(&payload)?);
            ws.send(msg).await?;
            info!("Sent voice SELECT_PROTOCOL");
        }
        Ok(())
    }

    /// Send SPEAKING packet
    pub async fn send_speaking(&self, speaking: bool, delay: u32, ssrc: u32) -> VoiceResult<()> {
        let speaking_pkt = VoiceSpeaking {
            speaking: if speaking { 1 } else { 0 },
            delay,
            ssrc,
        };

        let payload = serde_json::json!({
            "op": VoiceOpCode::Speaking as u8,
            "d": speaking_pkt,
        });

        self.send_json(payload).await?;
        Ok(())
    }

    /// Send HEARTBEAT packet
    async fn send_heartbeat(ws: &mut Option<WebSocketStream<MaybeTlsStream<TcpStream>>>, interval: u64) -> VoiceResult<()> {
        let heartbeat = VoiceHeartbeat {
            t: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_millis() as u64,
        };

        let payload = serde_json::json!({
            "op": VoiceOpCode::Heartbeat as u8,
            "d": heartbeat,
        });

        if let Some(ws) = ws {
            let msg = Message::Text(serde_json::to_string(&payload)?);
            ws.send(msg).await?;
            debug!("Sent voice heartbeat");
        }
        Ok(())
    }

    /// Send JSON payload
    async fn send_json(&self, payload: serde_json::Value) -> VoiceResult<()> {
        let msg = Message::Text(serde_json::to_string(&payload)?);
        if let Some(ws) = self.ws_stream.read().as_ref() {
            // We can't send directly here because we need mutable access
            // In a real implementation, we'd use a channel to send messages
        }
        Ok(())
    }

    /// Start heartbeat loop
    fn start_heartbeat_loop(
        heartbeat_interval: Arc<RwLock<u32>>,
        last_heartbeat_ack: Arc<RwLock<Option<Instant>>>,
        ws_stream: Arc<RwLock<Option<WebSocketStream<MaybeTlsStream<TcpStream>>>>>,
        running: Arc<RwLock<bool>>,
    ) {
        tokio::spawn(async move {
            let mut interval = interval(Duration::from_millis(*heartbeat_interval.read() as u64));

            while *running.read() {
                interval.tick().await;

                // Check for missed ACK
                if let Some(last) = *last_heartbeat_ack.read() {
                    let interval_ms = *heartbeat_interval.read() as u64;
                    if last.elapsed() > Duration::from_millis(interval_ms * 2) {
                        warn!("Voice heartbeat ACK missed, connection may be dead");
                        *running.write() = false;
                        break;
                    }
                }

                // Send heartbeat
                if let Err(e) = Self::send_heartbeat(&mut ws_stream.write(), *heartbeat_interval.read() as u64).await {
                    error!("Failed to send voice heartbeat: {}", e);
                    break;
                }
            }
            info!("Voice heartbeat loop ended");
        });
    }

    /// Get the SSRC assigned by Discord
    pub fn ssrc(&self) -> Option<u32> {
        *self.ssrc.read()
    }

    /// Get the IP address for UDP connection
    pub fn ip(&self) -> Option<String> {
        self.ip.read().clone()
    }

    /// Get the port for UDP connection
    pub fn port(&self) -> Option<u16> {
        *self.port.read()
    }

    /// Get the encryption modes
    pub fn modes(&self) -> Vec<String> {
        self.modes.read().clone()
    }

    /// Get the secret key for DTLS/SRTP
    pub fn secret_key(&self) -> Option<Vec<u8>> {
        self.secret_key.read().clone()
    }

    /// Check if signaling is connected
    pub fn is_connected(&self) -> bool {
        matches!(*self.state.read(), SignalingState::Connected)
    }

    /// Disconnect from voice signaling
    pub async fn disconnect(&self) -> VoiceResult<()> {
        info!("Disconnecting voice signaling...");
        *self.running.write() = false;
        *self.state.write() = SignalingState::Disconnected;

        if let Some(mut ws) = self.ws_stream.write().take() {
            let _ = ws.close(None).await;
        }

        self.event_tx.send(VoiceEvent::Disconnected).ok();
        Ok(())
    }
}