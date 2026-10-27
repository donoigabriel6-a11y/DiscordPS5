//! PS5 Audio System
//!
//! Handles audio capture (microphone) and playback. On Linux, uses cpal with JACK/PulseAudio.
//! On PS5/FreeBSD, provides stub implementations that will use native PS5 audio APIs.

#[cfg(feature = "linux-audio")]
use std::sync::{Arc, Mutex};
#[cfg(feature = "linux-audio")]
use std::collections::VecDeque;

#[cfg(feature = "linux-audio")]
use anyhow::{Context, Result};
#[cfg(feature = "linux-audio")]
use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
#[cfg(feature = "linux-audio")]
use cpal::{SampleFormat, SampleRate, StreamConfig, SupportedStreamConfigRange};
#[cfg(feature = "linux-audio")]
use parking_lot::RwLock;
#[cfg(feature = "linux-audio")]
use rubato::{Resampler, SincFixedIn, SincInterpolationParameters, SincInterpolationType, WindowFunction};
use tracing::{debug, error, info, warn};

use crate::{DiscordError, Result as PlatformResult};

/// Audio configuration constants
pub const SAMPLE_RATE: u32 = 48000;
pub const CHANNELS: u16 = 2; // Stereo
pub const FRAME_SIZE: usize = 960; // 20ms at 48kHz
pub const OPUS_FRAME_SIZE: usize = 960;

/// Audio sample type (f32 for processing, i16 for Opus)
pub type AudioSample = f32;

/// Audio buffer for capture/playback
#[derive(Debug, Clone)]
pub struct AudioBuffer {
    pub samples: Vec<AudioSample>,
    pub sample_rate: u32,
    pub channels: u16,
    pub timestamp: u64,
}

impl AudioBuffer {
    pub fn new(samples: Vec<AudioSample>, sample_rate: u32, channels: u16) -> Self {
        Self {
            samples,
            sample_rate,
            channels,
            timestamp: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_millis() as u64,
        }
    }

    pub fn silent(sample_rate: u32, channels: u16, frames: usize) -> Self {
        Self::new(vec![0.0; frames * channels as usize], sample_rate, channels)
    }

    pub fn len(&self) -> usize {
        self.samples.len() / self.channels as usize
    }

    pub fn is_empty(&self) -> bool {
        self.samples.is_empty()
    }
}

// Linux implementation using cpal
#[cfg(feature = "linux-audio")]
mod linux_audio {
    use super::*;

    /// Audio capture for microphone input
    pub struct AudioCapture {
        stream: Option<cpal::Stream>,
        config: StreamConfig,
        buffer: Arc<Mutex<VecDeque<AudioBuffer>>>,
        resampler: Option<SincFixedIn<AudioSample>>,
        running: Arc<std::sync::atomic::AtomicBool>,
        noise_suppression: bool,
        echo_cancellation: bool,
        auto_gain_control: bool,
    }

    impl AudioCapture {
        pub fn new(config: &crate::config::AudioConfig) -> PlatformResult<Self> {
            let host = cpal::default_host();

            // Find input device
            let device = if let Some(name) = &config.input_device {
                host.input_devices()?
                    .find(|d| d.name().map(|n| n.contains(name)).unwrap_or(false))
                    .ok_or_else(|| DiscordError::Audio(format!("Input device '{}' not found", name)))?
            } else {
                host.default_input_device()
                    .ok_or_else(|| DiscordError::Audio("No default input device".to_string()))?
            };

            info!("Using audio input device: {}", device.name()?);

            // Get supported config
            let supported_configs: Vec<_> = device.supported_input_configs()?.collect();
            let config_range = supported_configs
                .iter()
                .find(|c| c.channels() >= CHANNELS && c.sample_rate() >= SampleRate(SAMPLE_RATE))
                .ok_or_else(|| DiscordError::Audio("No suitable input config found".to_string()))?;

            let stream_config: StreamConfig = config_range.with_sample_rate(SampleRate(SAMPLE_RATE)).into();

            info!("Input config: {}Hz, {}ch, {:?}", stream_config.sample_rate.0, stream_config.channels, stream_config.sample_format);

            // Create resampler if needed
            let resampler = if stream_config.sample_rate.0 != SAMPLE_RATE {
                Some(Self::create_resampler(stream_config.sample_rate.0, SAMPLE_RATE)?)
            } else {
                None
            };

            Ok(Self {
                stream: None,
                config: stream_config,
                buffer: Arc::new(Mutex::new(VecDeque::with_capacity(10))),
                resampler,
                running: Arc::new(std::sync::atomic::AtomicBool::new(false)),
                noise_suppression: config.noise_suppression,
                echo_cancellation: config.echo_cancellation,
                auto_gain_control: config.auto_gain_control,
            })
        }

        fn create_resampler(from_rate: u32, to_rate: u32) -> PlatformResult<SincFixedIn<AudioSample>> {
            let params = SincInterpolationParameters {
                sinc_len: 256,
                f_cutoff: 0.95,
                interpolation: SincInterpolationType::Linear,
                oversampling_factor: 256,
                window: WindowFunction::BlackmanHarris2,
            };

            let resampler = SincFixedIn::<AudioSample>::new(
                to_rate as f64 / from_rate as f64,
                2.0,
                params,
                FRAME_SIZE,
                CHANNELS as usize,
            ).context("Failed to create resampler")?;

            Ok(resampler)
        }

        pub fn start(&mut self) -> PlatformResult<()> {
            if self.stream.is_some() {
                return Ok(());
            }

            let buffer = self.buffer.clone();
            let resampler = self.resampler.clone();
            let running = self.running.clone();
            let config = self.config.clone();
            let channels = config.channels as usize;

            running.store(true, std::sync::atomic::Ordering::Relaxed);

            let err_fn = move |err| {
                error!("Audio capture error: {}", err);
            };

            let data_fn = move |data: &[AudioSample], _: &cpal::InputCallbackInfo| {
                if !running.load(std::sync::atomic::Ordering::Relaxed) {
                    return;
                }

                let mut frames = data.len() / channels;
                let mut samples = data.to_vec();

                // Apply resampling if needed
                if let Some(ref mut resampler) = resampler {
                    let input: Vec<Vec<AudioSample>> = (0..channels)
                        .map(|ch| data.iter().skip(ch).step_by(channels).cloned().collect())
                        .collect();

                    match resampler.process(&input, None) {
                        Ok(output) => {
                            samples = output.into_iter().flatten().collect();
                            frames = samples.len() / channels;
                        }
                        Err(e) => {
                            warn!("Resampling error: {}", e);
                            return;
                        }
                    }
                }

                // Apply audio processing
                if frames > 0 {
                    // Process in chunks of FRAME_SIZE
                    for chunk in samples.chunks_mut(FRAME_SIZE * channels) {
                        let mut buf = VecDeque::new();
                        // Would apply noise suppression, AEC, AGC here
                        // For now just pass through

                        let audio_buf = AudioBuffer::new(chunk.to_vec(), SAMPLE_RATE, CHANNELS);

                        let mut buffer_lock = buffer.lock().unwrap();
                        if buffer_lock.len() >= 10 {
                            buffer_lock.pop_front();
                        }
                        buffer_lock.push_back(audio_buf);
                    }
                }
            };

            let stream = match config.sample_format {
                SampleFormat::F32 => device.build_input_stream(&config, data_fn, err_fn, None)?,
                SampleFormat::I16 => {
                    let data_fn_i16 = move |data: &[i16], _: &cpal::InputCallbackInfo| {
                        if !running.load(std::sync::atomic::Ordering::Relaxed) {
                            return;
                        }
                        let samples: Vec<AudioSample> = data.iter().map(|&s| s as f32 / i16::MAX as f32).collect();
                        // ... same processing
                    };
                    device.build_input_stream(&config, data_fn_i16, err_fn, None)?
                }
                SampleFormat::U16 => {
                    let data_fn_u16 = move |data: &[u16], _: &cpal::InputCallbackInfo| {
                        if !running.load(std::sync::atomic::Ordering::Relaxed) {
                            return;
                        }
                        let samples: Vec<AudioSample> = data.iter().map(|&s| (s as f32 - u16::MAX as f32 / 2.0) / (u16::MAX as f32 / 2.0)).collect();
                        // ... same processing
                    };
                    device.build_input_stream(&config, data_fn_u16, err_fn, None)?
                }
                _ => return Err(DiscordError::Audio("Unsupported sample format".to_string()).into()),
            };

            stream.play()?;
            self.stream = Some(stream);

            info!("Audio capture started");
            Ok(())
        }

        pub fn stop(&mut self) {
            self.running.store(false, std::sync::atomic::Ordering::Relaxed);
            self.stream = None;
            info!("Audio capture stopped");
        }

        pub fn read(&self) -> Option<AudioBuffer> {
            self.buffer.lock().unwrap().pop_front()
        }

        pub fn available_frames(&self) -> usize {
            self.buffer.lock().unwrap().iter().map(|b| b.len()).sum()
        }
    }

    impl Drop for AudioCapture {
        fn drop(&mut self) {
            self.stop();
        }
    }

    /// Audio playback for output
    pub struct AudioPlayback {
        stream: Option<cpal::Stream>,
        config: StreamConfig,
        buffer: Arc<Mutex<VecDeque<AudioBuffer>>>,
        resampler: Option<SincFixedIn<AudioSample>>,
        running: Arc<std::sync::atomic::AtomicBool>,
        volume: f32,
    }

    impl AudioPlayback {
        pub fn new(config: &crate::config::AudioConfig) -> PlatformResult<Self> {
            let host = cpal::default_host();

            let device = if let Some(name) = &config.output_device {
                host.output_devices()?
                    .find(|d| d.name().map(|n| n.contains(name)).unwrap_or(false))
                    .ok_or_else(|| DiscordError::Audio(format!("Output device '{}' not found", name)))?
            } else {
                host.default_output_device()
                    .ok_or_else(|| DiscordError::Audio("No default output device".to_string()))?
            };

            info!("Using audio output device: {}", device.name()?);

            let supported_configs: Vec<_> = device.supported_output_configs()?.collect();
            let config_range = supported_configs
                .iter()
                .find(|c| c.channels() >= CHANNELS && c.sample_rate() >= SampleRate(SAMPLE_RATE))
                .ok_or_else(|| DiscordError::Audio("No suitable output config found".to_string()))?;

            let stream_config: StreamConfig = config_range.with_sample_rate(SampleRate(SAMPLE_RATE)).into();

            info!("Output config: {}Hz, {}ch, {:?}", stream_config.sample_rate.0, stream_config.channels, stream_config.sample_format);

            let resampler = if stream_config.sample_rate.0 != SAMPLE_RATE {
                Some(Self::create_resampler(stream_config.sample_rate.0, SAMPLE_RATE)?)
            } else {
                None
            };

            Ok(Self {
                stream: None,
                config: stream_config,
                buffer: Arc::new(Mutex::new(VecDeque::with_capacity(20))),
                resampler,
                running: Arc::new(std::sync::atomic::AtomicBool::new(false)),
                volume: config.output_volume,
            })
        }

        fn create_resampler(from_rate: u32, to_rate: u32) -> PlatformResult<SincFixedIn<AudioSample>> {
            let params = SincInterpolationParameters {
                sinc_len: 256,
                f_cutoff: 0.95,
                interpolation: SincInterpolationType::Linear,
                oversampling_factor: 256,
                window: WindowFunction::BlackmanHarris2,
            };

            let resampler = SincFixedIn::<AudioSample>::new(
                to_rate as f64 / from_rate as f64,
                2.0,
                params,
                FRAME_SIZE,
                CHANNELS as usize,
            ).context("Failed to create resampler")?;

            Ok(resampler)
        }

        pub fn start(&mut self) -> PlatformResult<()> {
            if self.stream.is_some() {
                return Ok(());
            }

            let buffer = self.buffer.clone();
            let resampler = self.resampler.clone();
            let running = self.running.clone();
            let config = self.config.clone();
            let channels = config.channels as usize;
            let volume = Arc::new(RwLock::new(self.volume));

            running.store(true, std::sync::atomic::Ordering::Relaxed);

            let err_fn = move |err| {
                error!("Audio playback error: {}", err);
            };

            let data_fn = move |data: &mut [AudioSample], _: &cpal::OutputCallbackInfo| {
                if !running.load(std::sync::atomic::Ordering::Relaxed) {
                    data.fill(0.0);
                    return;
                }

                let vol = *volume.read();
                let needed = data.len();
                let mut filled = 0;

                while filled < needed {
                    let mut buffer_lock = buffer.lock().unwrap();

                    if let Some(mut audio_buf) = buffer_lock.pop_front() {
                        drop(buffer_lock); // Release lock before processing

                        // Apply resampling if needed
                        let samples = if let Some(ref mut resampler) = resampler {
                            let input: Vec<Vec<AudioSample>> = (0..CHANNELS as usize)
                                .map(|ch| audio_buf.samples.iter().skip(ch).step_by(CHANNELS as usize).cloned().collect())
                                .collect();

                            match resampler.process(&input, None) {
                                Ok(output) => output.into_iter().flatten().collect(),
                                Err(e) => {
                                    warn!("Playback resampling error: {}", e);
                                    vec![0.0; needed - filled]
                                }
                            }
                        } else {
                            audio_buf.samples
                        };

                        let to_copy = (needed - filled).min(samples.len());
                        for i in 0..to_copy {
                            data[filled + i] = samples[i] * vol;
                        }
                        filled += to_copy;

                        // If we didn't use all samples, put remainder back
                        if to_copy < samples.len() {
                            let remainder = AudioBuffer::new(
                                samples[to_copy..].to_vec(),
                                SAMPLE_RATE,
                                CHANNELS,
                            );
                            buffer.lock().unwrap().push_front(remainder);
                        }
                    } else {
                        // Buffer underrun - fill with silence
                        for i in filled..needed {
                            data[i] = 0.0;
                        }
                        filled = needed;
                    }
                }
            };

            let stream = match config.sample_format {
                SampleFormat::F32 => device.build_output_stream(&config, data_fn, err_fn, None)?,
                SampleFormat::I16 => {
                    let data_fn_i16 = move |data: &mut [i16], _: &cpal::OutputCallbackInfo| {
                        let mut f32_data = vec![0.0; data.len()];
                        data_fn(&mut f32_data, _);
                        for (i, &sample) in f32_data.iter().enumerate() {
                            data[i] = (sample.clamp(-1.0, 1.0) * i16::MAX as f32) as i16;
                        }
                    };
                    device.build_output_stream(&config, data_fn_i16, err_fn, None)?
                }
                SampleFormat::U16 => {
                    let data_fn_u16 = move |data: &mut [u16], _: &cpal::OutputCallbackInfo| {
                        let mut f32_data = vec![0.0; data.len()];
                        data_fn(&mut f32_data, _);
                        for (i, &sample) in f32_data.iter().enumerate() {
                            data[i] = ((sample.clamp(-1.0, 1.0) + 1.0) * 0.5 * u16::MAX as f32) as u16;
                        }
                    };
                    device.build_output_stream(&config, data_fn_u16, err_fn, None)?
                }
                _ => return Err(DiscordError::Audio("Unsupported sample format".to_string()).into()),
            };

            stream.play()?;
            self.stream = Some(stream);

            info!("Audio playback started");
            Ok(())
        }

        pub fn stop(&mut self) {
            self.running.store(false, std::sync::atomic::Ordering::Relaxed);
            self.stream = None;
            info!("Audio playback stopped");
        }

        pub fn write(&self, buffer: AudioBuffer) -> PlatformResult<()> {
            let mut buf_lock = self.buffer.lock().unwrap();
            if buf_lock.len() >= 20 {
                return Err(DiscordError::Audio("Playback buffer full".to_string()).into());
            }
            buf_lock.push_back(buffer);
            Ok(())
        }

        pub fn set_volume(&mut self, volume: f32) {
            self.volume = volume.clamp(0.0, 1.0);
        }

        pub fn buffered_duration_ms(&self) -> u64 {
            let buf_lock = self.buffer.lock().unwrap();
            let total_frames: usize = buf_lock.iter().map(|b| b.len()).sum();
            (total_frames as f64 / SAMPLE_RATE as f64 * 1000.0) as u64
        }
    }

    impl Drop for AudioPlayback {
        fn drop(&mut self) {
            self.stop();
        }
    }

    /// Combined audio system for voice chat
    pub struct VoiceAudioSystem {
        capture: Option<AudioCapture>,
        playback: Option<AudioPlayback>,
        config: crate::config::AudioConfig,
    }

    impl VoiceAudioSystem {
        pub fn new(config: crate::config::AudioConfig) -> PlatformResult<Self> {
            let capture = AudioCapture::new(&config)?;
            let playback = AudioPlayback::new(&config)?;

            Ok(Self {
                capture: Some(capture),
                playback: Some(playback),
                config,
            })
        }

        pub fn start(&mut self) -> PlatformResult<()> {
            if let Some(capture) = &mut self.capture {
                capture.start()?;
            }
            if let Some(playback) = &mut self.playback {
                playback.start()?
            }
            Ok(())
        }

        pub fn stop(&mut self) {
            if let Some(capture) = &mut self.capture {
                capture.stop();
            }
            if let Some(playback) = &mut self.playback {
                playback.stop();
            }
        }

        pub fn read_capture(&self) -> Option<AudioBuffer> {
            self.capture.as_ref()?.read()
        }

        pub fn write_playback(&self, buffer: AudioBuffer) -> PlatformResult<()> {
            self.playback.as_ref()?.write(buffer)
        }

        pub fn set_input_volume(&mut self, volume: f32) {
            self.config.input_volume = volume.clamp(0.0, 1.0);
        }

        pub fn set_output_volume(&mut self, volume: f32) {
            self.config.output_volume = volume.clamp(0.0, 1.0);
            if let Some(playback) = &mut self.playback {
                playback.set_volume(volume);
            }
        }
    }
}

// PS5/FreeBSD stub implementation
#[cfg(not(feature = "linux-audio"))]
mod ps5_audio {
    use super::*;

    /// Stub audio capture for PS5
    pub struct AudioCapture {
        running: bool,
    }

    impl AudioCapture {
        pub fn new(_config: &crate::config::AudioConfig) -> PlatformResult<Self> {
            info!("Initializing PS5 audio capture stub");
            Ok(Self { running: false })
        }

        pub fn start(&mut self) -> PlatformResult<()> {
            self.running = true;
            info!("PS5 audio capture started (stub)");
            Ok(())
        }

        pub fn stop(&mut self) {
            self.running = false;
            info!("PS5 audio capture stopped (stub)");
        }

        pub fn read(&self) -> Option<AudioBuffer> {
            None // No audio data in stub
        }

        pub fn available_frames(&self) -> usize {
            0
        }
    }

    impl Drop for AudioCapture {
        fn drop(&mut self) {
            self.stop();
        }
    }

    /// Stub audio playback for PS5
    pub struct AudioPlayback {
        running: bool,
        volume: f32,
    }

    impl AudioPlayback {
        pub fn new(_config: &crate::config::AudioConfig) -> PlatformResult<Self> {
            info!("Initializing PS5 audio playback stub");
            Ok(Self { running: false, volume: 1.0 })
        }

        pub fn start(&mut self) -> PlatformResult<()> {
            self.running = true;
            info!("PS5 audio playback started (stub)");
            Ok(())
        }

        pub fn stop(&mut self) {
            self.running = false;
            info!("PS5 audio playback stopped (stub)");
        }

        pub fn write(&self, _buffer: AudioBuffer) -> PlatformResult<()> {
            Ok(())
        }

        pub fn set_volume(&mut self, volume: f32) {
            self.volume = volume.clamp(0.0, 1.0);
        }

        pub fn buffered_duration_ms(&self) -> u64 {
            0
        }
    }

    impl Drop for AudioPlayback {
        fn drop(&mut self) {
            self.stop();
        }
    }

    /// Combined audio system stub for PS5
    pub struct VoiceAudioSystem {
        capture: Option<AudioCapture>,
        playback: Option<AudioPlayback>,
        config: crate::config::AudioConfig,
    }

    impl VoiceAudioSystem {
        pub fn new(config: crate::config::AudioConfig) -> PlatformResult<Self> {
            let capture = AudioCapture::new(&config)?;
            let playback = AudioPlayback::new(&config)?;

            Ok(Self {
                capture: Some(capture),
                playback: Some(playback),
                config,
            })
        }

        pub fn start(&mut self) -> PlatformResult<()> {
            if let Some(capture) = &mut self.capture {
                capture.start()?;
            }
            if let Some(playback) = &mut self.playback {
                playback.start()?;
            }
            Ok(())
        }

        pub fn stop(&mut self) {
            if let Some(capture) = &mut self.capture {
                capture.stop();
            }
            if let Some(playback) = &mut self.playback {
                playback.stop();
            }
        }

        pub fn read_capture(&self) -> Option<AudioBuffer> {
            None
        }

        pub fn write_playback(&self, _buffer: AudioBuffer) -> PlatformResult<()> {
            Ok(())
        }

        pub fn set_input_volume(&mut self, volume: f32) {
            self.config.input_volume = volume.clamp(0.0, 1.0);
        }

        pub fn set_output_volume(&mut self, volume: f32) {
            self.config.output_volume = volume.clamp(0.0, 1.0);
            if let Some(playback) = &mut self.playback {
                playback.set_volume(volume);
            }
        }
    }
}

// Re-export the appropriate implementation
#[cfg(feature = "linux-audio")]
pub use linux_audio::*;

#[cfg(not(feature = "linux-audio"))]
pub use ps5_audio::*;

/// Initialize audio system
/// On Linux, uses cpal. On PS5, returns stub implementation.
pub fn init_audio(config: &crate::config::AudioConfig) -> PlatformResult<VoiceAudioSystem> {
    #[cfg(feature = "linux-audio")]
    {
        linux_audio::VoiceAudioSystem::new(config.clone())
    }
    #[cfg(not(feature = "linux-audio"))]
    {
        info!("Initializing PS5 audio stub");
        ps5_audio::VoiceAudioSystem::new(config.clone())
    }
}