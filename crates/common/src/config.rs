//! Configuration management

use serde::{Deserialize, Serialize};
use std::path::PathBuf;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AppConfig {
    pub discord: DiscordConfig,
    pub ui: UiConfig,
    pub audio: AudioConfig,
    pub network: NetworkConfig,
    pub storage: StorageConfig,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DiscordConfig {
    pub token: Option<String>,
    pub gateway_url: String,
    pub api_url: String,
    pub intents: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UiConfig {
    pub theme: Theme,
    pub font_scale: f32,
    pub message_density: MessageDensity,
    pub show_timestamps: bool,
    pub compact_mode: bool,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub enum Theme {
    Dark,
    Light,
    System,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub enum MessageDensity {
    Compact,
    Cozy,
    Comfortable,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AudioConfig {
    pub input_device: Option<String>,
    pub output_device: Option<String>,
    pub input_volume: f32,
    pub output_volume: f32,
    pub noise_suppression: bool,
    pub echo_cancellation: bool,
    pub auto_gain_control: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NetworkConfig {
    pub proxy: Option<String>,
    pub dns_over_https: bool,
    pub connection_timeout_secs: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StorageConfig {
    pub cache_dir: PathBuf,
    pub log_dir: PathBuf,
    pub max_cache_size_mb: u64,
    pub max_log_size_mb: u64,
}

impl Default for AppConfig {
    fn default() -> Self {
        Self {
            discord: DiscordConfig {
                token: None,
                gateway_url: "wss://gateway.discord.gg".to_string(),
                api_url: "https://discord.com/api/v10".to_string(),
                intents: 0x0000FFFFFFFFFFFF, // All intents
            },
            ui: UiConfig {
                theme: Theme::Dark,
                font_scale: 1.0,
                message_density: MessageDensity::Cozy,
                show_timestamps: true,
                compact_mode: false,
            },
            audio: AudioConfig {
                input_device: None,
                output_device: None,
                input_volume: 1.0,
                output_volume: 1.0,
                noise_suppression: true,
                echo_cancellation: true,
                auto_gain_control: true,
            },
            network: NetworkConfig {
                proxy: None,
                dns_over_https: true,
                connection_timeout_secs: 30,
            },
            storage: StorageConfig {
                cache_dir: PathBuf::from("/data/discord-ps5/cache"),
                log_dir: PathBuf::from("/data/discord-ps5/logs"),
                max_cache_size_mb: 500,
                max_log_size_mb: 50,
            },
        }
    }
}

impl AppConfig {
    pub fn load() -> Result<Self, crate::error::DiscordError> {
        let config_path = Self::config_path();
        if config_path.exists() {
            let content = std::fs::read_to_string(&config_path)?;
            Ok(serde_json::from_str(&content)?)
        } else {
            Ok(Self::default())
        }
    }

    pub fn save(&self) -> Result<(), crate::error::DiscordError> {
        let config_path = Self::config_path();
        if let Some(parent) = config_path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        let content = serde_json::to_string_pretty(self)?;
        std::fs::write(config_path, content)?;
        Ok(())
    }

    fn config_path() -> PathBuf {
        PathBuf::from("/data/discord-ps5/config.json")
    }
}