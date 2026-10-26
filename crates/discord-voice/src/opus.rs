//! Opus Audio Codec Wrapper
//!
//! Provides Opus encoding and decoding for Discord voice audio.
//! Uses the opus crate for low-level encoding/decoding operations.

use std::sync::Arc;

use anyhow::{Context, Result};
use opus::{Application, Channels, Encoder, Decoder};
use tracing::{debug, error, info, warn};

use crate::{DiscordError, Result as VoiceResult};

/// Opus configuration constants for Discord voice
pub const OPUS_SAMPLE_RATE: u32 = 48000;
pub const OPUS_CHANNELS: usize = 2; // Stereo
pub const OPUS_FRAME_DURATION_MS: u32 = 20; // 20ms frames
pub const OPUS_FRAME_SIZE: usize = (OPUS_SAMPLE_RATE as usize * OPUS_FRAME_DURATION_MS as usize) / 1000; // 960 samples per channel
pub const OPUS_MAX_PACKET_SIZE: usize = 4000; // Max Opus packet size in bytes
pub const OPUS_BITRATE: i32 = 64000; // 64 kbps for stereo
pub const OPUS_COMPLEXITY: i32 = 10; // Highest complexity for best quality

/// Opus encoder wrapper
pub struct OpusEncoder {
    encoder: Encoder,
    frame_size: usize,
    channels: Channels,
}

impl OpusEncoder {
    /// Create a new Opus encoder for Discord voice
    pub fn new() -> VoiceResult<Self> {
        let channels = Channels::Stereo;
        let mut encoder = Encoder::new(OPUS_SAMPLE_RATE, channels, Application::Audio)
            .context("Failed to create Opus encoder")?;

        // Configure encoder for Discord voice
        encoder.set_bitrate(opus::Bitrate::Bits(OPUS_BITRATE))
            .context("Failed to set Opus bitrate")?;
        encoder.set_complexity(OPUS_COMPLEXITY)
            .context("Failed to set Opus complexity")?;
        encoder.set_inband_fec(true)
            .context("Failed to enable inband FEC")?;
        encoder.set_dtx(false)
            .context("Failed to disable DTX")?;
        encoder.set_packet_loss_perc(0)
            .context("Failed to set packet loss percentage")?;

        info!("Opus encoder created: {}Hz, {}ch, {}kbps, {}ms frames, FEC enabled",
            OPUS_SAMPLE_RATE, OPUS_CHANNELS, OPUS_BITRATE / 1000, OPUS_FRAME_DURATION_MS);

        Ok(Self {
            encoder,
            frame_size: OPUS_FRAME_SIZE,
            channels,
        })
    }

    /// Get the frame size in samples per channel
    pub fn frame_size(&self) -> usize {
        self.frame_size
    }

    /// Get the number of channels
    pub fn channels(&self) -> usize {
        self.channels as usize
    }

    /// Encode PCM audio to Opus
    ///
    /// Input: Interleaved PCM samples (i16) of length frame_size * channels
    /// Output: Opus packet bytes
    pub fn encode(&mut self, pcm: &[i16], output: &mut [u8]) -> VoiceResult<usize> {
        let channels = self.channels();
        if pcm.len() != self.frame_size * channels {
            return Err(DiscordError::Audio(format!(
                "Invalid PCM length: expected {}, got {}",
                self.frame_size * channels, pcm.len()
            )).into());
        }

        let encoded_len = self.encoder.encode(pcm, output)
            .context("Failed to encode Opus")?;

        debug!("Encoded Opus packet: {} bytes", encoded_len);
        Ok(encoded_len)
    }

    /// Encode PCM audio to Opus (allocating output buffer)
    pub fn encode_vec(&mut self, pcm: &[i16]) -> VoiceResult<Vec<u8>> {
        let mut output = vec![0u8; OPUS_MAX_PACKET_SIZE];
        let len = self.encode(pcm, &mut output)?;
        output.truncate(len);
        Ok(output)
    }

    /// Reset encoder state
    pub fn reset(&mut self) -> VoiceResult<()> {
        self.encoder.reset_state()
            .context("Failed to reset Opus encoder")?;
        Ok(())
    }
}

impl Default for OpusEncoder {
    fn default() -> Self {
        Self::new().expect("Failed to create default Opus encoder")
    }
}

/// Opus decoder wrapper
pub struct OpusDecoder {
    decoder: Decoder,
    frame_size: usize,
    channels: Channels,
}

impl OpusDecoder {
    /// Create a new Opus decoder for Discord voice
    pub fn new() -> VoiceResult<Self> {
        let channels = Channels::Stereo;
        let decoder = Decoder::new(OPUS_SAMPLE_RATE, channels)
            .context("Failed to create Opus decoder")?;

        info!("Opus decoder created: {}Hz, {}ch", OPUS_SAMPLE_RATE, OPUS_CHANNELS);

        Ok(Self {
            decoder,
            frame_size: OPUS_FRAME_SIZE,
            channels,
        })
    }

    /// Get the frame size in samples per channel
    pub fn frame_size(&self) -> usize {
        self.frame_size
    }

    /// Get the number of channels
    pub fn channels(&self) -> usize {
        self.channels as usize
    }

    /// Decode Opus packet to PCM audio
    ///
    /// Input: Opus packet bytes
    /// Output: Interleaved PCM samples (i16) of length frame_size * channels
    pub fn decode(&mut self, opus_data: &[u8], pcm: &mut [i16]) -> VoiceResult<usize> {
        let decoded_samples = self.decoder.decode(opus_data, pcm, false)
            .context("Failed to decode Opus")?;

        if decoded_samples != self.frame_size {
            warn!("Decoded frame size mismatch: expected {}, got {}",
                self.frame_size, decoded_samples);
        }

        debug!("Decoded Opus packet: {} samples per channel", decoded_samples);
        Ok(decoded_samples)
    }

    /// Decode Opus packet with PLC (Packet Loss Concealment)
    ///
    /// Called when a packet is lost - pass empty slice for opus_data
    pub fn decode_plc(&mut self, pcm: &mut [i16]) -> VoiceResult<usize> {
        let decoded_samples = self.decoder.decode(&[], pcm, false)
            .context("Failed to decode Opus (PLC)")?;

        debug!("Decoded Opus PLC: {} samples per channel", decoded_samples);
        Ok(decoded_samples)
    }

    /// Reset decoder state
    pub fn reset(&mut self) -> VoiceResult<()> {
        self.decoder.reset_state()
            .context("Failed to reset Opus decoder")?;
        Ok(())
    }
}

impl Default for OpusDecoder {
    fn default() -> Self {
        Self::new().expect("Failed to create default Opus decoder")
    }
}

/// Combined Opus codec for full-duplex voice
pub struct OpusCodec {
    encoder: OpusEncoder,
    decoder: OpusDecoder,
}

impl OpusCodec {
    /// Create a new Opus codec pair
    pub fn new() -> VoiceResult<Self> {
        let encoder = OpusEncoder::new()?;
        let decoder = OpusDecoder::new()?;
        Ok(Self { encoder, decoder })
    }

    /// Get mutable reference to encoder
    pub fn encoder(&mut self) -> &mut OpusEncoder {
        &mut self.encoder
    }

    /// Get mutable reference to decoder
    pub fn decoder(&mut self) -> &mut OpusDecoder {
        &mut self.decoder
    }

    /// Get frame size
    pub fn frame_size(&self) -> usize {
        self.encoder.frame_size()
    }

    /// Get channels
    pub fn channels(&self) -> usize {
        self.encoder.channels()
    }

    /// Encode PCM to Opus
    pub fn encode(&mut self, pcm: &[i16], output: &mut [u8]) -> VoiceResult<usize> {
        self.encoder.encode(pcm, output)
    }

    /// Decode Opus to PCM
    pub fn decode(&mut self, opus_data: &[u8], pcm: &mut [i16]) -> VoiceResult<usize> {
        self.decoder.decode(opus_data, pcm)
    }

    /// Decode with PLC
    pub fn decode_plc(&mut self, pcm: &mut [i16]) -> VoiceResult<usize> {
        self.decoder.decode_plc(pcm)
    }

    /// Reset both encoder and decoder
    pub fn reset(&mut self) -> VoiceResult<()> {
        self.encoder.reset()?;
        self.decoder.reset()?;
        Ok(())
    }
}

/// Opus audio buffer for processing
#[derive(Debug, Clone)]
pub struct OpusAudioBuffer {
    /// Interleaved PCM samples (i16)
    pub samples: Vec<i16>,
    /// Sample rate (always 48000 for Discord)
    pub sample_rate: u32,
    /// Number of channels (always 2 for Discord)
    pub channels: usize,
    /// Timestamp in milliseconds
    pub timestamp: u64,
    /// Sequence number for RTP
    pub sequence: u16,
}

impl OpusAudioBuffer {
    /// Create a new audio buffer with silence
    pub fn silent(frame_size: usize, channels: usize) -> Self {
        Self {
            samples: vec![0; frame_size * channels],
            sample_rate: OPUS_SAMPLE_RATE,
            channels,
            timestamp: 0,
            sequence: 0,
        }
    }

    /// Create from PCM samples
    pub fn from_pcm(samples: Vec<i16>, channels: usize) -> Self {
        Self {
            samples,
            sample_rate: OPUS_SAMPLE_RATE,
            channels,
            timestamp: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_millis() as u64,
            sequence: 0,
        }
    }

    /// Get number of frames (samples per channel)
    pub fn frames(&self) -> usize {
        self.samples.len() / self.channels
    }

    /// Check if buffer is empty
    pub fn is_empty(&self) -> bool {
        self.samples.is_empty()
    }

    /// Get duration in milliseconds
    pub fn duration_ms(&self) -> u32 {
        (self.frames() as f64 / self.sample_rate as f64 * 1000.0) as u32
    }
}

/// Convert f32 audio samples to i16 (for Opus encoding)
pub fn f32_to_i16(samples: &[f32], output: &mut [i16]) {
    for (i, &sample) in samples.iter().enumerate() {
        if i >= output.len() { break; }
        output[i] = (sample.clamp(-1.0, 1.0) * i16::MAX as f32) as i16;
    }
}

/// Convert i16 audio samples to f32 (for audio processing)
pub fn i16_to_f32(samples: &[i16], output: &mut [f32]) {
    for (i, &sample) in samples.iter().enumerate() {
        if i >= output.len() { break; }
        output[i] = sample as f32 / i16::MAX as f32;
    }
}

/// Audio resampler for converting between sample rates
pub struct AudioResampler {
    ratio: f64,
    // In a real implementation, this would use rubato or similar
}

impl AudioResampler {
    pub fn new(from_rate: u32, to_rate: u32) -> Self {
        Self {
            ratio: to_rate as f64 / from_rate as f64,
        }
    }

    /// Resample audio (placeholder - would use rubato in real implementation)
    pub fn resample(&self, input: &[f32], output: &mut [f32]) -> usize {
        // Simple linear interpolation placeholder
        let input_len = input.len();
        let output_len = output.len().min((input_len as f64 * self.ratio) as usize);

        for i in 0..output_len {
            let src_idx = i as f64 / self.ratio;
            let idx = src_idx as usize;
            let frac = (src_idx - idx as f64) as f32;

            if idx + 1 < input_len {
                output[i] = input[idx] * (1.0 - frac) + input[idx + 1] * frac;
            } else if idx < input_len {
                output[i] = input[idx];
            } else {
                output[i] = 0.0;
            }
        }

        output_len
    }
}