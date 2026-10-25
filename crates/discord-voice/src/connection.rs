//! Voice WebRTC Connection
//!
//! Main voice connection that integrates signaling, ICE, audio, and Opus.

use std::sync::Arc;
use std::collections::HashMap;
use std::net::SocketAddr;
use std::time::Duration;

use anyhow::{Context, Result};
use parking_lot::RwLock;
use tokio::sync::mpsc;
use tokio::time::{interval, timeout};
use tracing::{debug, error, info, warn};
use webrtc::api::interceptor_registry::register_default_interceptors;
use webrtc::api::media_engine::MediaEngine;
use webrtc::api::APIBuilder;
use webrtc::ice_transport::ice_candidate::RTCIceCandidateInit;
use webrtc::ice_transport::ice_server::RTCIceServer;
use webrtc::interceptor::registry::Registry;
use webrtc::peer_connection::configuration::RTCConfiguration;
use webrtc::peer_connection::peer_connection_state::RTCPeerConnectionState;
use webrtc::peer_connection::RTCPeerConnection;
use webrtc::rtp_transceiver::rtp_receiver::RTCRtpReceiver;
use webrtc::track::track_remote::TrackRemote;
use webrtc::track::track_local::{TrackLocal, TrackLocalWriter};
use webrtc::track::track_local::TrackLocalStaticRTP;

use crate::{DiscordError, Result as VoiceResult, VoiceConfig, VoiceEvent, VoiceConnectionState};
use crate::audio::{AudioSendPipeline, AudioReceivePipeline, RtpPacket, SrtpContext};
use crate::opus::{OpusCodec, OPUS_FRAME_SIZE, OPUS_CHANNELS};
use crate::ice::{IceManager, IceEvent, default_ice_servers};
use crate::signaling::{VoiceSignaling, SignalingState};
use discord_common::discord_types::*;

/// Voice connection manager that integrates all components
pub struct VoiceConnection {
    config: VoiceConfig,
    state: Arc<RwLock<VoiceConnectionState>>,
    peer_connection: Arc<RwLock<Option<Arc<RTCPeerConnection>>>>,
    audio_track: Arc<RwLock<Option<Arc<TrackLocalStaticRTP>>>>,
    remote_tracks: Arc<RwLock<HashMap<u32, Arc<TrackRemote>>>>,
    ssrc: Arc<RwLock<Option<u32>>>,
    event_tx: mpsc::UnboundedSender<VoiceEvent>,
    speaking: Arc<RwLock<bool>>,

    // New components
    signaling: Arc<RwLock<Option<VoiceSignaling>>>,
    ice_manager: Arc<RwLock<Option<IceManager>>>,
    send_pipeline: Arc<RwLock<Option<AudioSendPipeline>>>,
    recv_pipeline: Arc<RwLock<Option<AudioReceivePipeline>>>,
    opus_codec: Arc<RwLock<Option<OpusCodec>>>,
}

impl VoiceConnection {
    pub fn new(config: VoiceConfig) -> (Self, mpsc::UnboundedReceiver<VoiceEvent>) {
        let (tx, rx) = mpsc::unbounded_channel();

        let conn = Self {
            config,
            state: Arc::new(RwLock::new(VoiceConnectionState::Disconnected)),
            peer_connection: Arc::new(RwLock::new(None)),
            audio_track: Arc::new(RwLock::new(None)),
            remote_tracks: Arc::new(RwLock::new(HashMap::new())),
            ssrc: Arc::new(RwLock::new(None)),
            event_tx: tx,
            speaking: Arc::new(RwLock::new(false)),
            signaling: Arc::new(RwLock::new(None)),
            ice_manager: Arc::new(RwLock::new(None)),
            send_pipeline: Arc::new(RwLock::new(None)),
            recv_pipeline: Arc::new(RwLock::new(None)),
            opus_codec: Arc::new(RwLock::new(None)),
        };

        (conn, rx)
    }

    /// Connect to voice channel - full handshake
    pub async fn connect(&self) -> VoiceResult<()> {
        info!("Connecting to voice channel...");
        *self.state.write() = VoiceConnectionState::Connecting;

        // 1. Create WebRTC peer connection
        let peer_connection = self.create_peer_connection().await?;
        *self.peer_connection.write() = Some(peer_connection.clone());

        // 2. Create local audio track
        let audio_track = self.create_audio_track()?;
        *self.audio_track.write() = Some(audio_track.clone());

        // 3. Add audio track to peer connection
        let _sender = peer_connection.add_track(audio_track.clone() as Arc<dyn TrackLocal + Send + Sync>).await
            .context("Failed to add audio track")?;

        // 4. Set up track event handlers
        self.setup_track_handlers(peer_connection.clone()).await;

        // 5. Initialize ICE manager
        let ice_event_tx = self.event_tx.clone();
        let ice_manager = IceManager::new(peer_connection.clone(), ice_event_tx);
        ice_manager.initialize(default_ice_servers()).await?;
        ice_manager.start_monitoring();
        *self.ice_manager.write() = Some(ice_manager);

        // 6. Create Opus codec
        let opus_codec = OpusCodec::new()?;
        *self.opus_codec.write() = Some(opus_codec);

        // 7. Create and connect signaling
        let signaling = VoiceSignaling::new(self.config.clone(), self.event_tx.clone());
        signaling.connect().await?;
        *self.signaling.write() = Some(signaling);

        // 8. Wait for signaling to complete and get connection info
        let (ssrc, ip, port, secret_key) = self.wait_for_signaling_ready().await?;

        // 9. Set up SRTP context
        let srtp = SrtpContext::new(&secret_key, ssrc)?;

        // 10. Create audio pipelines
        let mut send_pipeline = AudioSendPipeline::new(ssrc)?;
        send_pipeline.set_srtp(srtp.clone());
        *self.send_pipeline.write() = Some(send_pipeline);

        let mut recv_pipeline = AudioReceivePipeline::new(ssrc, 100, 50)?;
        recv_pipeline.set_srtp(srtp);
        *self.recv_pipeline.write() = Some(recv_pipeline);

        // 11. Update SSRC
        *self.ssrc.write() = Some(ssrc);

        // 12. Start ICE connectivity (create offer/answer)
        self.start_ice_negotiation(peer_connection.clone()).await?;

        *self.state.write() = VoiceConnectionState::Connected;
        info!("Voice connection established!");

        // Start audio processing loops
        self.start_audio_loops();

        Ok(())
    }

    /// Wait for signaling to complete and return connection parameters
    async fn wait_for_signaling_ready(&self) -> VoiceResult<(u32, String, u16, Vec<u8>)> {
        let signaling = self.signaling.read().clone()
            .ok_or_else(|| DiscordError::VoiceConnection("Signaling not initialized".to_string()))?;

        let start = std::time::Instant::now();
        let timeout_dur = Duration::from_secs(30);

        while start.elapsed() < timeout_dur {
            tokio::time::sleep(Duration::from_millis(100)).await;

            if signaling.is_connected() {
                let ssrc = signaling.ssrc()
                    .ok_or_else(|| DiscordError::VoiceConnection("No SSRC from signaling".to_string()))?;
                let ip = signaling.ip()
                    .ok_or_else(|| DiscordError::VoiceConnection("No IP from signaling".to_string()))?;
                let port = signaling.port()
                    .ok_or_else(|| DiscordError::VoiceConnection("No port from signaling".to_string()))?;
                let secret_key = signaling.secret_key()
                    .ok_or_else(|| DiscordError::VoiceConnection("No secret key from signaling".to_string()))?;

                return Ok((ssrc, ip, port, secret_key));
            }

            if matches!(signaling.state.read(), SignalingState::Failed) {
                return Err(DiscordError::VoiceConnection("Signaling failed".to_string()).into());
            }
        }

        Err(DiscordError::VoiceConnection("Signaling timeout".to_string()).into())
    }

    /// Create WebRTC peer connection
    async fn create_peer_connection(&self) -> VoiceResult<Arc<RTCPeerConnection>> {
        let mut media_engine = MediaEngine::default();

        // Register Opus codec
        media_engine.register_default_codecs()
            .context("Failed to register default codecs")?;

        let mut registry = Registry::new();
        registry = register_default_interceptors(registry, &mut media_engine)
            .context("Failed to register interceptors")?;

        let api = APIBuilder::new()
            .with_media_engine(media_engine)
            .with_interceptor_registry(registry)
            .build();

        let config = RTCConfiguration {
            ice_servers: self.config.ice_servers.clone(),
            ..Default::default()
        };

        let pc = Arc::new(api.new_peer_connection(config).await
            .context("Failed to create peer connection")?);

        Ok(pc)
    }

    /// Create local audio track for Opus
    fn create_audio_track(&self) -> VoiceResult<Arc<TrackLocalStaticRTP>> {
        let track = Arc::new(TrackLocalStaticRTP::new(
            webrtc::rtp::codec::RTCRtpCodecCapability {
                mime_type: webrtc::rtp::codec::MIME_TYPE_OPUS.to_string(),
                clock_rate: 48000,
                channels: 2,
                sdp_fmtp_line: "minptime=10;useinbandfec=1".to_string(),
                ..Default::default()
            },
            "audio".to_string(),
            "discord-ps5".to_string(),
        ));

        Ok(track)
    }

    /// Set up handlers for remote tracks
    async fn setup_track_handlers(&self, pc: Arc<RTCPeerConnection>) {
        let remote_tracks = self.remote_tracks.clone();
        let ssrc = self.ssrc.clone();
        let event_tx = self.event_tx.clone();

        pc.on_track(Box::new(move |track, receiver, _| {
            if let Ok(remote_track) = track.downcast_arc::<TrackRemote>() {
                let ssrc_val = remote_track.ssrc();
                *ssrc.write() = Some(ssrc_val);
                remote_tracks.write().insert(ssrc_val, remote_track.clone());

                info!("Remote track added: SSRC={}", ssrc_val);

                let event_tx = event_tx.clone();
                tokio::spawn(async move {
                    Self::handle_remote_track(remote_track, event_tx).await;
                });
            }
            Box::pin(async {})
        }));

        pc.on_peer_connection_state_change(Box::new(move |state| {
            info!("Peer connection state: {:?}", state);
            Box::pin(async {})
        }));

        pc.on_ice_connection_state_change(Box::new(move |state| {
            info!("ICE connection state: {:?}", state);
            Box::pin(async {})
        }));
    }

    /// Handle incoming remote audio track
    async fn handle_remote_track(track: Arc<TrackRemote>, event_tx: mpsc::UnboundedSender<VoiceEvent>) {
        let ssrc = track.ssrc();
        let mut buf = vec![0u8; 4096];

        loop {
            match track.read(&mut buf).await {
                Ok((n, _)) => {
                    let data = buf[..n].to_vec();
                    event_tx.send(VoiceEvent::AudioReceived { ssrc, data }).ok();
                }
                Err(e) => {
                    error!("Error reading from remote track {}: {}", ssrc, e);
                    break;
                }
            }
        }
    }

    /// Start ICE negotiation (create offer)
    async fn start_ice_negotiation(&self, pc: Arc<RTCPeerConnection>) -> VoiceResult<()> {
        let offer = pc.create_offer(None).await
            .context("Failed to create offer")?;

        pc.set_local_description(offer.clone()).await
            .context("Failed to set local description")?;

        // In a real implementation, we'd send this offer via signaling
        // For now, we'll use the WebRTC ICE which handles it automatically
        info!("ICE negotiation started (WebRTC handles offer/answer automatically)");

        // Wait for ICE to complete
        if let Some(ice_manager) = self.ice_manager.read().as_ref() {
            ice_manager.wait_for_completion(Duration::from_secs(30)).await?;
        }

        Ok(())
    }

    /// Start audio processing loops
    fn start_audio_loops(&self) {
        // Audio send loop (read from mic, encode, send via UDP)
        self.start_send_loop();

        // Audio receive loop (read from UDP, decode, play)
        self.start_receive_loop();
    }

    /// Audio send loop
    fn start_send_loop(&self) {
        let send_pipeline = self.send_pipeline.clone();
        let event_tx = self.event_tx.clone();
        let speaking = self.speaking.clone();
        let config = self.config.clone();

        tokio::spawn(async move {
            // In a real implementation, this would read from the audio capture device
            // For now, this is a placeholder that would be connected to the audio system
            info!("Audio send loop started (placeholder)");
        });
    }

    /// Audio receive loop
    fn start_receive_loop(&self) {
        let recv_pipeline = self.recv_pipeline.clone();
        let event_tx = self.event_tx.clone();

        tokio::spawn(async move {
            // In a real implementation, this would read from the UDP socket
            // For now, this is a placeholder
            info!("Audio receive loop started (placeholder)");
        });
    }

    /// Write audio data from microphone
    pub async fn write_audio(&self, pcm_data: &[i16]) -> VoiceResult<()> {
        if let Some(pipeline) = self.send_pipeline.read().as_ref() {
            let packets = pipeline.encode_and_packetize(pcm_data)?;
            // In a real implementation, send packets via UDP socket
            for packet in packets {
                // Send via UDP to the voice server
                debug!("Sending audio packet: {} bytes", packet.len());
            }
        }
        Ok(())
    }

    /// Set speaking state (sends SPEAKING packet via signaling)
    pub async fn set_speaking(&self, speaking: bool) -> VoiceResult<()> {
        *self.speaking.write() = speaking;

        if let (Some(signaling), Some(ssrc)) = (
            self.signaling.read().as_ref(),
            *self.ssrc.read()
        ) {
            signaling.send_speaking(speaking, 0, ssrc).await?;
        }

        Ok(())
    }

    /// Get current connection state
    pub fn state(&self) -> VoiceConnectionState {
        *self.state.read()
    }

    /// Check if connected
    pub fn is_connected(&self) -> bool {
        matches!(*self.state.read(), VoiceConnectionState::Connected)
    }

    /// Disconnect from voice
    pub async fn disconnect(&self) -> VoiceResult<()> {
        info!("Disconnecting from voice...");
        *self.state.write() = VoiceConnectionState::Disconnected;

        // Stop pipelines
        // (In a real implementation, we'd signal the loops to stop)

        // Disconnect signaling
        if let Some(signaling) = self.signaling.read().as_ref() {
            signaling.disconnect().await.ok();
        }

        // Close peer connection
        if let Some(pc) = self.peer_connection.write().take() {
            pc.close().await.ok();
        }

        self.event_tx.send(VoiceEvent::Disconnected).ok();
        Ok(())
    }
}