//! Audio RTP/SRTP Handling
//!
//! Handles RTP packet construction, parsing, SRTP encryption/decryption,
//! jitter buffer, and audio pipeline for Discord voice.

use std::sync::Arc;
use std::collections::VecDeque;
use std::time::{Duration, Instant};

use anyhow::{Context, Result};
use aes::cipher::{KeyIvInit, StreamCipher};
use aes::Aes128;
use ctr::Ctr128BE;
use hmac::{Hmac, Mac};
use sha1::Sha1;
use parking_lot::RwLock;
use tokio::sync::mpsc;
use tracing::{debug, error, info, warn};

use crate::{DiscordError, Result as VoiceResult, OpusAudioBuffer, OPUS_FRAME_SIZE, OPUS_SAMPLE_RATE, OPUS_CHANNELS};
use crate::opus::{OpusEncoder, OpusDecoder};

/// RTP header size (bytes)
const RTP_HEADER_SIZE: usize = 12;
/// RTP extension header for contributing sources (CSRC)
const RTP_EXT_HEADER_SIZE: usize = 4;
/// SRTP authentication tag size (HMAC-SHA1-80)
const SRTP_AUTH_TAG_SIZE: usize = 10;
/// SRTP master key size (128-bit)
const SRTP_MASTER_KEY_SIZE: usize = 16;
/// SRTP master salt size (112-bit = 14 bytes)
const SRTP_MASTER_SALT_SIZE: usize = 14;

/// RTP packet structure
#[derive(Debug, Clone)]
pub struct RtpPacket {
    pub version: u8,        // 2 bits
    pub padding: bool,      // 1 bit
    pub extension: bool,    // 1 bit
    pub csrc_count: u8,     // 4 bits
    pub marker: bool,       // 1 bit
    pub payload_type: u8,   // 7 bits
    pub sequence: u16,      // 16 bits
    pub timestamp: u32,     // 32 bits
    pub ssrc: u32,          // 32 bits
    pub csrc_list: Vec<u32>, // CSRC identifiers
    pub extension_header: Option<RtpExtensionHeader>,
    pub payload: Vec<u8>,   // Opus data
}

/// RTP extension header (one-byte header)
#[derive(Debug, Clone)]
pub struct RtpExtensionHeader {
    pub profile: u16,       // 0xBEDE for one-byte header
    pub length: u16,        // length in 32-bit words minus 1
    pub extensions: Vec<RtpExtension>,
}

/// RTP one-byte extension
#[derive(Debug, Clone)]
pub struct RtpExtension {
    pub id: u8,             // 4 bits
    pub len: u8,            // 4 bits (length in bytes - 1)
    pub data: Vec<u8>,
}

impl RtpPacket {
    /// Create a new RTP packet for Opus audio
    pub fn new_opus(sequence: u16, timestamp: u32, ssrc: u32, payload: Vec<u8>) -> Self {
        Self {
            version: 2,
            padding: false,
            extension: false,
            csrc_count: 0,
            marker: false, // Set to true for last packet of a talk spurt
            payload_type: 0x78, // Dynamic payload type for Opus (typically 100-127)
            sequence,
            timestamp,
            ssrc,
            csrc_list: Vec::new(),
            extension_header: None,
            payload,
        }
    }

    /// Serialize RTP packet to bytes
    pub fn serialize(&self) -> Vec<u8> {
        let mut buf = Vec::with_capacity(RTP_HEADER_SIZE + self.payload.len());

        // Byte 0: version (2) | padding (1) | extension (1) | csrc_count (4)
        let byte0 = (self.version << 6) |
            ((self.padding as u8) << 5) |
            ((self.extension as u8) << 4) |
            self.csrc_count;
        buf.push(byte0);

        // Byte 1: marker (1) | payload_type (7)
        let byte1 = ((self.marker as u8) << 7) | self.payload_type;
        buf.push(byte1);

        // Sequence number (big endian)
        buf.extend_from_slice(&self.sequence.to_be_bytes());

        // Timestamp (big endian)
        buf.extend_from_slice(&self.timestamp.to_be_bytes());

        // SSRC (big endian)
        buf.extend_from_slice(&self.ssrc.to_be_bytes());

        // CSRC list
        for csrc in &self.csrc_list {
            buf.extend_from_slice(&csrc.to_be_bytes());
        }

        // Extension header (if present)
        if let Some(ext) = &self.extension_header {
            buf.extend_from_slice(&ext.profile.to_be_bytes());
            buf.extend_from_slice(&ext.length.to_be_bytes());
            for e in &ext.extensions {
                let hdr = (e.id << 4) | e.len;
                buf.push(hdr);
                buf.extend_from_slice(&e.data);
            }
        }

        // Payload
        buf.extend_from_slice(&self.payload);

        buf
    }

    /// Parse RTP packet from bytes
    pub fn parse(data: &[u8]) -> VoiceResult<Self> {
        if data.len() < RTP_HEADER_SIZE {
            return Err(DiscordError::Audio("RTP packet too short".to_string()).into());
        }

        let byte0 = data[0];
        let version = byte0 >> 6;
        let padding = (byte0 >> 5) & 1 != 0;
        let extension = (byte0 >> 4) & 1 != 0;
        let csrc_count = byte0 & 0x0F;

        let byte1 = data[1];
        let marker = byte1 >> 7 != 0;
        let payload_type = byte1 & 0x7F;

        let sequence = u16::from_be_bytes([data[2], data[3]]);
        let timestamp = u32::from_be_bytes([data[4], data[5], data[6], data[7]]);
        let ssrc = u32::from_be_bytes([data[8], data[9], data[10], data[11]]);

        let mut offset = RTP_HEADER_SIZE;
        let mut csrc_list = Vec::new();
        for _ in 0..csrc_count {
            if offset + 4 > data.len() {
                return Err(DiscordError::Audio("RTP packet truncated in CSRC".to_string()).into());
            }
            csrc_list.push(u32::from_be_bytes([
                data[offset], data[offset+1], data[offset+2], data[offset+3]
            ]));
            offset += 4;
        }

        let mut extension_header = None;
        if extension {
            if offset + 4 > data.len() {
                return Err(DiscordError::Audio("RTP packet truncated in extension".to_string()).into());
            }
            let profile = u16::from_be_bytes([data[offset], data[offset+1]]);
            let length = u16::from_be_bytes([data[offset+2], data[offset+3]]);
            offset += 4;

            let ext_bytes = (length + 1) * 4;
            if offset + ext_bytes as usize > data.len() {
                return Err(DiscordError::Audio("RTP packet truncated in extension data".to_string()).into());
            }

            let mut extensions = Vec::new();
            let mut ext_offset = offset;
            while ext_offset < offset + ext_bytes as usize {
                if ext_offset >= data.len() { break; }
                let id = data[ext_offset] >> 4;
                let len = (data[ext_offset] & 0x0F) + 1;
                ext_offset += 1;
                if ext_offset + len as usize > data.len() { break; }
                extensions.push(RtpExtension {
                    id,
                    len: len - 1,
                    data: data[ext_offset..ext_offset + len as usize].to_vec(),
                });
                ext_offset += len as usize;
            }

            extension_header = Some(RtpExtensionHeader { profile, length, extensions });
            offset += ext_bytes as usize;
        }

        let payload = data[offset..].to_vec();

        Ok(Self {
            version,
            padding,
            extension,
            csrc_count,
            marker,
            payload_type,
            sequence,
            timestamp,
            ssrc,
            csrc_list,
            extension_header,
            payload,
        })
    }
}

/// SRTP context for encryption/decryption
pub struct SrtpContext {
    master_key: [u8; SRTP_MASTER_KEY_SIZE],
    master_salt: [u8; SRTP_MASTER_SALT_SIZE],
    ssrc: u32,
    roc: u32, // Roll-over counter
    srtp_session_key: [u8; SRTP_MASTER_KEY_SIZE],
    srtp_session_salt: [u8; SRTP_MASTER_SALT_SIZE],
    srtp_auth_key: [u8; 20], // HMAC-SHA1 key
}

impl SrtpContext {
    /// Create SRTP context from Discord secret key
    /// secret_key is 32 bytes: 16 bytes key + 14 bytes salt + 2 bytes (ignored)
    pub fn new(secret_key: &[u8], ssrc: u32) -> VoiceResult<Self> {
        if secret_key.len() < SRTP_MASTER_KEY_SIZE + SRTP_MASTER_SALT_SIZE {
            return Err(DiscordError::Audio("Secret key too short".to_string()).into());
        }

        let mut master_key = [0u8; SRTP_MASTER_KEY_SIZE];
        master_key.copy_from_slice(&secret_key[0..SRTP_MASTER_KEY_SIZE]);

        let mut master_salt = [0u8; SRTP_MASTER_SALT_SIZE];
        master_salt.copy_from_slice(&secret_key[SRTP_MASTER_KEY_SIZE..SRTP_MASTER_KEY_SIZE + SRTP_MASTER_SALT_SIZE]);

        // Derive session keys using AES-CM KDF (simplified)
        let (session_key, session_salt, auth_key) = Self::derive_keys(&master_key, &master_salt, ssrc)?;

        Ok(Self {
            master_key,
            master_salt,
            ssrc,
            roc: 0,
            srtp_session_key: session_key,
            srtp_session_salt: session_salt,
            srtp_auth_key: auth_key,
        })
    }

    /// Derive SRTP session keys from master key/salt (AES-CM PRF)
    fn derive_keys(master_key: &[u8; 16], master_salt: &[u8; 14], ssrc: u32)
        -> VoiceResult<([u8; 16], [u8; 14], [u8; 20])>
    {
        // Simplified key derivation - in production use proper AES-CM KDF
        // This is a placeholder implementation
        let mut session_key = *master_key;
        let mut session_salt = *master_salt;
        let mut auth_key = [0u8; 20];

        // XOR SSRC into salt for session-specific keys
        let ssrc_bytes = ssrc.to_be_bytes();
        for i in 0..4 {
            if i < master_salt.len() {
                session_salt[i] ^= ssrc_bytes[i];
            }
        }

        // Generate auth key from master key (simplified)
        let mut mac = Hmac::<Sha1>::new_from_slice(master_key)
            .map_err(|e| DiscordError::Audio(format!("HMAC init failed: {}", e)))?;
        mac.update(&ssrc.to_be_bytes());
        auth_key.copy_from_slice(&mac.finalize().into_bytes());

        Ok((session_key, session_salt, auth_key))
    }

    /// Build IV for AES-CTR (per RFC 3711)
    fn build_iv(&self, sequence: u16) -> [u8; 16] {
        let mut iv = [0u8; 16];
        iv[0..14].copy_from_slice(&self.srtp_session_salt);
        iv[14] = (sequence >> 8) as u8;
        iv[15] = sequence as u8;
        // XOR ROC into first 4 bytes of IV
        let roc_bytes = self.roc.to_be_bytes();
        for i in 0..4 {
            iv[i] ^= roc_bytes[i];
        }
        iv
    }

    /// Encrypt RTP payload in-place (SRTP)
    pub fn encrypt_rtp(&mut self, packet: &mut RtpPacket) -> VoiceResult<Vec<u8>> {
        let iv = self.build_iv(packet.sequence);

        // AES-CTR encryption
        let mut cipher = Ctr128BE::<Aes128>::new(&self.srtp_session_key.into(), &iv.into());
        let mut payload = packet.payload.clone();
        cipher.apply_keystream(&mut payload);

        // Build authenticated data (RTP header without extensions)
        let mut auth_data = Vec::new();
        let byte0 = (packet.version << 6) |
            ((packet.padding as u8) << 5) |
            ((packet.extension as u8) << 4) |
            packet.csrc_count;
        auth_data.push(byte0);
        let byte1 = ((packet.marker as u8) << 7) | packet.payload_type;
        auth_data.push(byte1);
        auth_data.extend_from_slice(&packet.sequence.to_be_bytes());
        auth_data.extend_from_slice(&packet.timestamp.to_be_bytes());
        auth_data.extend_from_slice(&packet.ssrc.to_be_bytes());
        for csrc in &packet.csrc_list {
            auth_data.extend_from_slice(&csrc.to_be_bytes());
        }

        // Add ROC to auth data
        auth_data.extend_from_slice(&self.roc.to_be_bytes());

        // Compute HMAC-SHA1 authentication tag
        let mut mac = Hmac::<Sha1>::new_from_slice(&self.srtp_auth_key)
            .map_err(|e| DiscordError::Audio(format!("HMAC init failed: {}", e)))?;
        mac.update(&auth_data);
        mac.update(&payload);
        let tag = mac.finalize().into_bytes();
        let auth_tag = &tag[0..SRTP_AUTH_TAG_SIZE];

        // Append auth tag to payload
        let mut encrypted_payload = payload;
        encrypted_payload.extend_from_slice(auth_tag);

        packet.payload = encrypted_payload;

        // Update ROC if sequence wrapped
        if packet.sequence == 0xFFFF {
            self.roc += 1;
        }

        Ok(packet.serialize())
    }

    /// Decrypt SRTP payload
    pub fn decrypt_rtp(&mut self, data: &[u8]) -> VoiceResult<RtpPacket> {
        if data.len() < RTP_HEADER_SIZE + SRTP_AUTH_TAG_SIZE {
            return Err(DiscordError::Audio("SRTP packet too short".to_string()).into());
        }

        // Parse header first (unencrypted)
        let packet = RtpPacket::parse(&data[0..data.len() - SRTP_AUTH_TAG_SIZE])?;

        // Extract auth tag
        let auth_tag = &data[data.len() - SRTP_AUTH_TAG_SIZE..];
        let encrypted_payload = &data[RTP_HEADER_SIZE + packet.csrc_list.len() * 4 +
            packet.extension_header.as_ref().map_or(0, |e| 4 + (e.length + 1) * 4)..data.len() - SRTP_AUTH_TAG_SIZE];

        let iv = self.build_iv(packet.sequence);

        // Build authenticated data
        let mut auth_data = Vec::new();
        let byte0 = (packet.version << 6) |
            ((packet.padding as u8) << 5) |
            ((packet.extension as u8) << 4) |
            packet.csrc_count;
        auth_data.push(byte0);
        let byte1 = ((packet.marker as u8) << 7) | packet.payload_type;
        auth_data.push(byte1);
        auth_data.extend_from_slice(&packet.sequence.to_be_bytes());
        auth_data.extend_from_slice(&packet.timestamp.to_be_bytes());
        auth_data.extend_from_slice(&packet.ssrc.to_be_bytes());
        for csrc in &packet.csrc_list {
            auth_data.extend_from_slice(&csrc.to_be_bytes());
        }
        auth_data.extend_from_slice(&self.roc.to_be_bytes());

        // Verify auth tag
        let mut mac = Hmac::<Sha1>::new_from_slice(&self.srtp_auth_key)
            .map_err(|e| DiscordError::Audio(format!("HMAC init failed: {}", e)))?;
        mac.update(&auth_data);
        mac.update(encrypted_payload);
        let computed_tag = mac.finalize().into_bytes();

        if &computed_tag[0..SRTP_AUTH_TAG_SIZE] != auth_tag {
            return Err(DiscordError::Audio("SRTP authentication failed".to_string()).into());
        }

        // Decrypt payload
        let mut cipher = Ctr128BE::<Aes128>::new(&self.srtp_session_key.into(), &iv.into());
        let mut payload = encrypted_payload.to_vec();
        cipher.apply_keystream(&mut payload);

        let mut decrypted_packet = packet;
        decrypted_packet.payload = payload;

        if packet.sequence == 0xFFFF {
            self.roc += 1;
        }

        Ok(decrypted_packet)
    }
}

/// Jitter buffer for incoming audio packets
pub struct JitterBuffer {
    buffer: VecDeque<JitterBufferEntry>,
    capacity: usize,
    target_latency_ms: u32,
    last_played_seq: Option<u16>,
    clock_offset: i64,
}

#[derive(Debug, Clone)]
struct JitterBufferEntry {
    packet: RtpPacket,
    received_at: Instant,
    sequence: u16,
}

impl JitterBuffer {
    pub fn new(capacity: usize, target_latency_ms: u32) -> Self {
        Self {
            buffer: VecDeque::with_capacity(capacity),
            capacity,
            target_latency_ms,
            last_played_seq: None,
            clock_offset: 0,
        }
    }

    /// Add packet to jitter buffer
    pub fn push(&mut self, packet: RtpPacket) {
        let sequence = packet.sequence;
        let entry = JitterBufferEntry {
            packet,
            received_at: Instant::now(),
            sequence,
        };

        // Insert in sequence order - find position first, then insert
        let mut insert_pos = self.buffer.len();
        for i in 0..self.buffer.len() {
            if self.buffer[i].sequence > sequence {
                insert_pos = i;
                break;
            }
        }
        self.buffer.insert(insert_pos, entry);

        // Trim if over capacity
        while self.buffer.len() > self.capacity {
            self.buffer.pop_front();
        }
    }

    /// Get next packet for playout
    pub fn pop(&mut self) -> Option<RtpPacket> {
        if self.buffer.is_empty() {
            return None;
        }

        // Simple playout: return earliest packet if it's time
        let now = Instant::now();
        if let Some(entry) = self.buffer.front() {
            let latency = now.duration_since(entry.received_at).as_millis() as u32;
            if latency >= self.target_latency_ms || self.last_played_seq.is_none() {
                let entry = self.buffer.pop_front()?;
                self.last_played_seq = Some(entry.sequence);
                return Some(entry.packet);
            }
        }

        None
    }

    /// Get packet with PLC (Packet Loss Concealment) if needed
    pub fn get_next(&mut self) -> Option<RtpPacket> {
        if let Some(packet) = self.pop() {
            Some(packet)
        } else if let Some(last_seq) = self.last_played_seq {
            // Generate PLC packet (silence)
            let plc_seq = last_seq.wrapping_add(1);
            let plc_packet = RtpPacket::new_opus(
                plc_seq,
                0, // timestamp will be set by caller
                0, // ssrc will be set by caller
                vec![0; 1], // minimal Opus frame (PLC)
            );
            self.last_played_seq = Some(plc_seq);
            Some(plc_packet)
        } else {
            None
        }
    }

    pub fn len(&self) -> usize {
        self.buffer.len()
    }

    pub fn is_empty(&self) -> bool {
        self.buffer.is_empty()
    }
}

/// Audio pipeline for sending (encode -> RTP -> SRTP -> UDP)
pub struct AudioSendPipeline {
    encoder: OpusEncoder,
    sequence: Arc<RwLock<u16>>,
    timestamp: Arc<RwLock<u32>>,
    ssrc: u32,
    srtp: Option<SrtpContext>,
}

impl AudioSendPipeline {
    pub fn new(ssrc: u32) -> VoiceResult<Self> {
        let encoder = OpusEncoder::new()?;
        Ok(Self {
            encoder,
            sequence: Arc::new(RwLock::new(0)),
            timestamp: Arc::new(RwLock::new(0)),
            ssrc,
            srtp: None,
        })
    }

    pub fn set_srtp(&mut self, srtp: SrtpContext) {
        self.srtp = Some(srtp);
    }

    /// Encode PCM audio and create encrypted RTP packets
    pub fn encode_and_packetize(&mut self, pcm: &[i16]) -> VoiceResult<Vec<Vec<u8>>> {
        if pcm.len() % (OPUS_FRAME_SIZE * OPUS_CHANNELS) != 0 {
            return Err(DiscordError::Audio("PCM length must be multiple of frame size".to_string()).into());
        }

        let mut packets = Vec::new();
        let frames = pcm.len() / (OPUS_FRAME_SIZE * OPUS_CHANNELS);

        for i in 0..frames {
            let start = i * OPUS_FRAME_SIZE * OPUS_CHANNELS;
            let end = start + OPUS_FRAME_SIZE * OPUS_CHANNELS;
            let frame_pcm = &pcm[start..end];

            // Encode to Opus
            let mut opus_data = vec![0u8; 4000];
            let opus_len = self.encoder.encode(frame_pcm, &mut opus_data)?;
            opus_data.truncate(opus_len);

            // Create RTP packet
            let mut seq = self.sequence.write();
            let mut ts = self.timestamp.write();

            let mut packet = RtpPacket::new_opus(*seq, *ts, self.ssrc, opus_data);

            // Encrypt if SRTP available
            if let Some(srtp) = &mut self.srtp {
                let encrypted = srtp.encrypt_rtp(&mut packet)?;
                packets.push(encrypted);
            } else {
                packets.push(packet.serialize());
            }

            *seq = seq.wrapping_add(1);
            *ts = ts.wrapping_add(OPUS_FRAME_SIZE as u32);
        }

        Ok(packets)
    }
}

/// Audio pipeline for receiving (UDP -> SRTP -> RTP -> decode -> PCM)
pub struct AudioReceivePipeline {
    decoder: OpusDecoder,
    ssrc: u32,
    srtp: Option<SrtpContext>,
    jitter_buffer: JitterBuffer,
    last_timestamp: u32,
}

impl AudioReceivePipeline {
    pub fn new(ssrc: u32, jitter_capacity: usize, target_latency_ms: u32) -> VoiceResult<Self> {
        let decoder = OpusDecoder::new()?;
        Ok(Self {
            decoder,
            ssrc,
            srtp: None,
            jitter_buffer: JitterBuffer::new(jitter_capacity, target_latency_ms),
            last_timestamp: 0,
        })
    }

    pub fn set_srtp(&mut self, srtp: SrtpContext) {
        self.srtp = Some(srtp);
    }

    /// Process incoming UDP packet and return decoded PCM if ready
    pub fn process_packet(&mut self, data: &[u8]) -> VoiceResult<Option<OpusAudioBuffer>> {
        // Decrypt SRTP
        let packet = if let Some(srtp) = &mut self.srtp {
            srtp.decrypt_rtp(data)?
        } else {
            RtpPacket::parse(data)?
        };

        // Add to jitter buffer
        self.jitter_buffer.push(packet);

        // Try to get next packet for playout
        if let Some(rtp_packet) = self.jitter_buffer.get_next() {
            // Decode Opus
            let mut pcm = vec![0i16; OPUS_FRAME_SIZE * OPUS_CHANNELS];
            let decoded = self.decoder.decode(&rtp_packet.payload, &mut pcm)?;

            if decoded > 0 {
                return Ok(Some(OpusAudioBuffer {
                    samples: pcm,
                    sample_rate: OPUS_SAMPLE_RATE,
                    channels: OPUS_CHANNELS,
                    timestamp: rtp_packet.timestamp as u64,
                    sequence: rtp_packet.sequence,
                }));
            }
        }

        Ok(None)
    }

    pub fn jitter_buffer_len(&self) -> usize {
        self.jitter_buffer.len()
    }
}