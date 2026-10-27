//! ICE (Interactive Connectivity Establishment) Handling
//!
//! Provides ICE candidate gathering, connectivity checks, and NAT traversal
//! for Discord voice WebRTC connections.

use std::sync::Arc;
use std::collections::HashMap;
use std::net::{IpAddr, Ipv4Addr, SocketAddr};
use std::time::Duration;

use anyhow::{Context, Result};
use parking_lot::RwLock;
use tokio::sync::mpsc;
use tracing::{debug, info, warn};
use webrtc::ice_transport::ice_candidate::RTCIceCandidateInit;
use webrtc::ice_transport::ice_server::RTCIceServer;
use webrtc::peer_connection::RTCPeerConnection;

use crate::{DiscordError, Result as VoiceResult};

/// ICE candidate type
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum IceCandidateType {
    Host,
    ServerReflexive,
    PeerReflexive,
    Relay,
}

/// ICE candidate
#[derive(Debug, Clone)]
pub struct IceCandidate {
    pub candidate: String,
    pub sdp_mid: Option<String>,
    pub sdp_mline_index: u16,
    pub username_fragment: Option<String>,
    pub candidate_type: IceCandidateType,
    pub priority: u32,
    pub address: IpAddr,
    pub port: u16,
    pub protocol: String, // "udp" or "tcp"
    pub related_address: Option<IpAddr>,
    pub related_port: Option<u16>,
}

/// ICE connection state
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum IceConnectionState {
    New,
    Checking,
    Connected,
    Completed,
    Failed,
    Disconnected,
    Closed,
}

/// ICE manager for handling ICE negotiation
pub struct IceManager {
    peer_connection: Arc<RTCPeerConnection>,
    state: Arc<RwLock<IceConnectionState>>,
    local_candidates: Arc<RwLock<Vec<IceCandidate>>>,
    remote_candidates: Arc<RwLock<Vec<IceCandidate>>>,
    selected_pair: Arc<RwLock<Option<(IceCandidate, IceCandidate)>>>,
    event_tx: mpsc::UnboundedSender<IceEvent>,
    running: Arc<RwLock<bool>>,
}

/// ICE events
#[derive(Debug, Clone)]
pub enum IceEvent {
    StateChanged(IceConnectionState),
    LocalCandidate(IceCandidate),
    RemoteCandidate(IceCandidate),
    SelectedPair(IceCandidate, IceCandidate),
    ConnectivityCheckResult { success: bool, local: IceCandidate, remote: IceCandidate },
    Error(String),
}

impl IceManager {
    /// Create a new ICE manager
    pub fn new(
        peer_connection: Arc<RTCPeerConnection>,
        event_tx: mpsc::UnboundedSender<IceEvent>,
    ) -> Self {
        Self {
            peer_connection,
            state: Arc::new(RwLock::new(IceConnectionState::New)),
            local_candidates: Arc::new(RwLock::new(Vec::new())),
            remote_candidates: Arc::new(RwLock::new(Vec::new())),
            selected_pair: Arc::new(RwLock::new(None)),
            event_tx,
            running: Arc::new(RwLock::new(false)),
        }
    }

    /// Initialize ICE with STUN/TURN servers
    pub async fn initialize(&self, ice_servers: Vec<RTCIceServer>) -> VoiceResult<()> {
        info!("Initializing ICE with {} servers", ice_servers.len());

        // Set up ICE event handlers
        self.setup_ice_handlers().await;

        *self.state.write() = IceConnectionState::Checking;
        self.emit_event(IceEvent::StateChanged(IceConnectionState::Checking));

        Ok(())
    }

    /// Set up ICE event handlers on the peer connection
    async fn setup_ice_handlers(&self) {
        let state = self.state.clone();
        let local_candidates = self.local_candidates.clone();
        let event_tx = self.event_tx.clone();
        let running = self.running.clone();

        // ICE candidate handler
        self.peer_connection.on_ice_candidate(Box::new(move |candidate: Option<RTCIceCandidateInit>| {
            if let Some(candidate) = candidate {
                if let Some(ice_candidate) = parse_ice_candidate(&candidate) {
                    info!("Local ICE candidate: {}", ice_candidate.candidate);
                    local_candidates.write().push(ice_candidate.clone());
                    event_tx.send(IceEvent::LocalCandidate(ice_candidate)).ok();
                }
            } else {
                info!("ICE candidate gathering complete");
            }
            Box::pin(async {})
        }));

        // ICE connection state change handler
        self.peer_connection.on_ice_connection_state_change(Box::new(move |state: webrtc::ice_transport::ice_connection_state::RTCIceConnectionState| {
            let ice_state = match state {
                webrtc::ice_transport::ice_connection_state::RTCIceConnectionState::New => IceConnectionState::New,
                webrtc::ice_transport::ice_connection_state::RTCIceConnectionState::Checking => IceConnectionState::Checking,
                webrtc::ice_transport::ice_connection_state::RTCIceConnectionState::Connected => IceConnectionState::Connected,
                webrtc::ice_transport::ice_connection_state::RTCIceConnectionState::Completed => IceConnectionState::Completed,
                webrtc::ice_transport::ice_connection_state::RTCIceConnectionState::Failed => IceConnectionState::Failed,
                webrtc::ice_transport::ice_connection_state::RTCIceConnectionState::Disconnected => IceConnectionState::Disconnected,
                webrtc::ice_transport::ice_connection_state::RTCIceConnectionState::Closed => IceConnectionState::Closed,
            };

            info!("ICE connection state: {:?}", ice_state);
            *state.write() = ice_state;
            event_tx.send(IceEvent::StateChanged(ice_state)).ok();

            Box::pin(async {})
        }));

        // ICE gathering state change handler
        self.peer_connection.on_ice_gathering_state_change(Box::new(move |state: webrtc::ice_transport::ice_gatherer_state::RTCIceGathererState| {
            info!("ICE gathering state: {:?}", state);
            Box::pin(async {})
        }));

        // Connection state change handler
        self.peer_connection.on_peer_connection_state_change(Box::new(move |state: webrtc::peer_connection::peer_connection_state::RTCPeerConnectionState| {
            info!("Peer connection state: {:?}", state);
            Box::pin(async {})
        }));
    }

    /// Add a remote ICE candidate
    pub async fn add_remote_candidate(&self, candidate: IceCandidate) -> VoiceResult<()> {
        info!("Adding remote ICE candidate: {}", candidate.candidate);

        let rtc_candidate = RTCIceCandidateInit {
            candidate: candidate.candidate.clone(),
            sdp_mid: candidate.sdp_mid,
            sdp_mline_index: Some(candidate.sdp_mline_index),
            username_fragment: candidate.username_fragment,
        };

        self.peer_connection.add_ice_candidate(rtc_candidate).await
            .context("Failed to add remote ICE candidate")?;

        self.remote_candidates.write().push(candidate.clone());
        self.emit_event(IceEvent::RemoteCandidate(candidate));

        Ok(())
    }

    /// Get current ICE connection state
    pub fn state(&self) -> IceConnectionState {
        *self.state.read()
    }

    /// Get local candidates
    pub fn local_candidates(&self) -> Vec<IceCandidate> {
        self.local_candidates.read().clone()
    }

    /// Get remote candidates
    pub fn remote_candidates(&self) -> Vec<IceCandidate> {
        self.remote_candidates.read().clone()
    }

    /// Get selected candidate pair
    pub fn selected_pair(&self) -> Option<(IceCandidate, IceCandidate)> {
        self.selected_pair.read().clone()
    }

    /// Wait for ICE connection to complete
    pub async fn wait_for_completion(&self, timeout: Duration) -> VoiceResult<IceConnectionState> {
        let start = std::time::Instant::now();

        while start.elapsed() < timeout {
            tokio::time::sleep(Duration::from_millis(100)).await;

            let state = *self.state.read();
            match state {
                IceConnectionState::Connected | IceConnectionState::Completed => {
                    return Ok(state);
                }
                IceConnectionState::Failed | IceConnectionState::Closed => {
                    return Err(DiscordError::VoiceConnection("ICE connection failed".to_string()).into());
                }
                _ => {}
            }
        }

        Err(DiscordError::VoiceConnection("ICE connection timeout".to_string()).into())
    }

    /// Start connectivity monitoring
    pub fn start_monitoring(&self) {
        *self.running.write() = true;

        let state = self.state.clone();
        let running = self.running.clone();
        let event_tx = self.event_tx.clone();
        let peer_connection = self.peer_connection.clone();

        tokio::spawn(async move {
            let mut interval = tokio::time::interval(Duration::from_secs(5));

            while *running.read() {
                interval.tick().await;

                let conn_state = peer_connection.ice_connection_state();
                let ice_state = match conn_state {
                    webrtc::ice_transport::ice_connection_state::RTCIceConnectionState::New => IceConnectionState::New,
                    webrtc::ice_transport::ice_connection_state::RTCIceConnectionState::Checking => IceConnectionState::Checking,
                    webrtc::ice_transport::ice_connection_state::RTCIceConnectionState::Connected => IceConnectionState::Connected,
                    webrtc::ice_transport::ice_connection_state::RTCIceConnectionState::Completed => IceConnectionState::Completed,
                    webrtc::ice_transport::ice_connection_state::RTCIceConnectionState::Failed => IceConnectionState::Failed,
                    webrtc::ice_transport::ice_connection_state::RTCIceConnectionState::Disconnected => IceConnectionState::Disconnected,
                    webrtc::ice_transport::ice_connection_state::RTCIceConnectionState::Closed => IceConnectionState::Closed,
                    webrtc::ice_transport::ice_connection_state::RTCIceConnectionState::Unspecified => IceConnectionState::New,
                };

                if ice_state != *state.read() {
                    *state.write() = ice_state;
                    event_tx.send(IceEvent::StateChanged(ice_state)).ok();

                    if matches!(ice_state, IceConnectionState::Failed | IceConnectionState::Disconnected) {
                        warn!("ICE connection degraded: {:?}", ice_state);
                    }
                }
            }
        });
    }

    /// Stop monitoring
    pub fn stop_monitoring(&self) {
        *self.running.write() = false;
    }

    /// Emit an ICE event
    fn emit_event(&self, event: IceEvent) {
        self.event_tx.send(event).ok();
    }
}

/// Parse WebRTC ICE candidate into our format
fn parse_ice_candidate(candidate: &RTCIceCandidateInit) -> Option<IceCandidate> {
    // Parse the candidate string (format: "candidate:1 1 udp 2122260223 192.168.1.100 54400 typ host")
    let parts: Vec<&str> = candidate.candidate.split_whitespace().collect();

    if parts.len() < 8 || parts[0] != "candidate:" {
        return None;
    }

    // Parse foundation, component, protocol, priority
    let _foundation = parts[1];
    let _component: u16 = parts[2].parse().ok()?;
    let protocol = parts[3].to_string();
    let priority = parts[4].parse().ok()?;
    let address = parts[5].parse().ok()?;
    let port = parts[6].parse().ok()?;

    // Find "typ" keyword
    let mut candidate_type = IceCandidateType::Host;
    let mut related_address = None;
    let mut related_port = None;

    for i in 7..parts.len() {
        match parts[i] {
            "typ" => {
                if i + 1 < parts.len() {
                    candidate_type = match parts[i + 1] {
                        "host" => IceCandidateType::Host,
                        "srflx" => IceCandidateType::ServerReflexive,
                        "prflx" => IceCandidateType::PeerReflexive,
                        "relay" => IceCandidateType::Relay,
                        _ => IceCandidateType::Host,
                    };
                }
            }
            "raddr" => {
                if i + 1 < parts.len() {
                    related_address = parts[i + 1].parse().ok();
                }
            }
            "rport" => {
                if i + 1 < parts.len() {
                    related_port = parts[i + 1].parse().ok();
                }
            }
            _ => {}
        }
    }

    Some(IceCandidate {
        candidate: candidate.candidate.clone(),
        sdp_mid: candidate.sdp_mid.clone(),
        sdp_mline_index: candidate.sdp_mline_index.unwrap_or(0),
        username_fragment: candidate.username_fragment.clone(),
        candidate_type,
        priority,
        address,
        port,
        protocol,
        related_address,
        related_port,
    })
}

/// Create default ICE servers for Discord voice
pub fn default_ice_servers() -> Vec<RTCIceServer> {
    vec![
        RTCIceServer {
            urls: vec!["stun:stun.l.google.com:19302".to_string()],
            ..Default::default()
        },
        RTCIceServer {
            urls: vec!["stun:stun1.l.google.com:19302".to_string()],
            ..Default::default()
        },
        RTCIceServer {
            urls: vec!["stun:stun2.l.google.com:19302".to_string()],
            ..Default::default()
        },
    ]
}

/// Create ICE servers with TURN support (for production)
pub fn ice_servers_with_turn(turn_url: &str, username: &str, credential: &str) -> Vec<RTCIceServer> {
    let mut servers = default_ice_servers();
    servers.push(RTCIceServer {
        urls: vec![turn_url.to_string()],
        username: username.to_string(),
        credential: credential.to_string(),
        // credential_type is private in webrtc v0.11, omit it
        ..Default::default()
    });
    servers
}