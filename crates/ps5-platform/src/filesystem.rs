//! PS5 Filesystem Abstraction

use std::fs;
use std::path::{Path, PathBuf};
use std::sync::OnceLock;

use anyhow::{Context, Result};
use tracing::{debug, info, warn};

use crate::{DiscordError, Result as PlatformResult};

/// Get the base data directory for the app
static DATA_DIR: OnceLock<PathBuf> = OnceLock::new();

pub fn init_data_dir() -> PlatformResult<PathBuf> {
    let dir = PathBuf::from("/data/discord-ps5");
    fs::create_dir_all(&dir)
        .context("Failed to create data directory")?;
    fs::create_dir_all(dir.join("cache"))?;
    fs::create_dir_all(dir.join("logs"))?;
    fs::create_dir_all(dir.join("config"))?;

    DATA_DIR.set(dir.clone()).ok();
    info!("Data directory initialized at {:?}", dir);
    Ok(dir)
}

pub fn data_dir() -> PathBuf {
    DATA_DIR.get().cloned().unwrap_or_else(|| PathBuf::from("/data/discord-ps5"))
}

pub fn cache_dir() -> PathBuf {
    data_dir().join("cache")
}

pub fn log_dir() -> PathBuf {
    data_dir().join("logs")
}

pub fn config_dir() -> PathBuf {
    data_dir().join("config")
}

/// Save data to app storage
pub fn save_file<P: AsRef<Path>>(relative_path: P, data: &[u8]) -> PlatformResult<()> {
    let path = data_dir().join(relative_path);
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
    fs::write(&path, data)
        .with_context(|| format!("Failed to write file {:?}", path))?;
    Ok(())
}

/// Load data from app storage
pub fn load_file<P: AsRef<Path>>(relative_path: P) -> PlatformResult<Vec<u8>> {
    let path = data_dir().join(relative_path);
    fs::read(&path)
        .with_context(|| format!("Failed to read file {:?}", path))
}

/// Delete a file
pub fn delete_file<P: AsRef<Path>>(relative_path: P) -> PlatformResult<()> {
    let path = data_dir().join(relative_path);
    if path.exists() {
        fs::remove_file(&path)
            .with_context(|| format!("Failed to delete file {:?}", path))?;
    }
    Ok(())
}

/// List files in a directory
pub fn list_files<P: AsRef<Path>>(relative_path: P) -> PlatformResult<Vec<PathBuf>> {
    let path = data_dir().join(relative_path);
    let mut files = Vec::new();

    if path.exists() {
        for entry in fs::read_dir(&path)? {
            let entry = entry?;
            files.push(entry.path());
        }
    }

    Ok(files)
}

/// Get file size
pub fn file_size<P: AsRef<Path>>(relative_path: P) -> PlatformResult<u64> {
    let path = data_dir().join(relative_path);
    let metadata = fs::metadata(&path)?;
    Ok(metadata.len())
}

/// Check if file exists
pub fn file_exists<P: AsRef<Path>>(relative_path: P) -> bool {
    data_dir().join(relative_path).exists()
}

/// Read JSON file
pub fn read_json<T: serde::de::DeserializeOwned, P: AsRef<Path>>(relative_path: P) -> PlatformResult<T> {
    let data = load_file(relative_path)?;
    serde_json::from_slice(&data)
        .context("Failed to parse JSON")
}

/// Write JSON file
pub fn write_json<T: serde::Serialize, P: AsRef<Path>>(relative_path: P, value: &T) -> PlatformResult<()> {
    let data = serde_json::to_vec_pretty(value)?;
    save_file(relative_path, &data)
}

/// Clean old cache files
pub fn clean_cache(max_age_days: u64, max_size_mb: u64) -> PlatformResult<u64> {
    let cache = cache_dir();
    let mut total_freed = 0u64;
    let now = std::time::SystemTime::now();
    let max_age = std::time::Duration::from_secs(max_age_days * 24 * 60 * 60);

    // Collect files with metadata
    let mut entries = Vec::new();
    for entry in fs::read_dir(&cache)? {
        let entry = entry?;
        let metadata = entry.metadata()?;
        let modified = metadata.modified().unwrap_or(now);
        if let Ok(age) = now.duration_since(modified) {
            if age > max_age {
                entries.push((entry.path(), metadata.len()));
            }
        }
    }

    // Sort by age (oldest first)
    entries.sort_by_key(|(_, size)| *size);

    // Delete until under size limit
    let max_size = max_size_mb * 1024 * 1024;
    let current_size: u64 = entries.iter().map(|(_, s)| *s).sum();

    if current_size > max_size {
        let mut to_delete = current_size - max_size;
        for (path, size) in entries {
            if to_delete == 0 {
                break;
            }
            fs::remove_file(&path)?;
            total_freed += size;
            to_delete = to_delete.saturating_sub(size);
        }
    }

    info!("Cache cleaned: freed {} bytes", total_freed);
    Ok(total_freed)
}