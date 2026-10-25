//! Gateway Heartbeat Management

use std::sync::Arc;
use std::time::{Duration, Instant};

use anyhow::{Context, Result};
use parking_lot::RwLock;
use tokio::time::interval;
use tracing::{debug, error, info, warn};

use crate::{GatewayConnection, DiscordError, Result as GatewayResult};

/// Heartbeat manager for maintaining gateway connection
pub struct HeartbeatManager {
    connection: Arc<GatewayConnection>,
    interval_ms: Arc<RwLock<u64>>,
    last_ack: Arc<RwLock<Option<Instant>>>,
    running: Arc<RwLock<bool>>,
    task_handle: Arc<tokio::sync::Mutex<Option<tokio::task::JoinHandle<()>>>>,
}

impl HeartbeatManager {
    pub fn new(connection: Arc<GatewayConnection>) -> Self {
        Self {
            connection,
            interval_ms: Arc::new(RwLock::new(41250)),
            last_ack: Arc::new(RwLock::new(None)),
            running: Arc::new(RwLock::new(false)),
            task_handle: Arc::new(tokio::sync::Mutex::new(None)),
        }
    }

    /// Start the heartbeat loop
    pub async fn start(&self) -> GatewayResult<()> {
        if *self.running.read() {
            return Ok(());
        }

        *self.running.write() = true;
        let interval_ms = *self.interval_ms.read();

        let connection = self.connection.clone();
        let interval_ms = self.interval_ms.clone();
        let last_ack = self.last_ack.clone();
        let running = self.running.clone();

        let handle = tokio::spawn(async move {
            let mut interval = interval(Duration::from_millis(interval_ms));

            while *running.read() {
                interval.tick().await;

                // Check if we missed an ACK
                if let Some(last) = *last_ack.read() {
                    if last.elapsed() > Duration::from_millis(*interval_ms.read() * 2) {
                        warn!("Heartbeat ACK missed, connection may be dead");
                        // Trigger reconnect
                        break;
                    }
                }

                // Send heartbeat
                if let Err(e) = connection.send_heartbeat().await {
                    error!("Failed to send heartbeat: {}", e);
                    break;
                }

                debug!("Sent heartbeat");
            }

            info!("Heartbeat loop ended");
        });

        *self.task_handle.lock().await = Some(handle);
        info!("Heartbeat manager started");
        Ok(())
    }

    /// Stop the heartbeat loop
    pub async fn stop(&self) {
        *self.running.write() = false;

        if let Some(handle) = self.task_handle.lock().await.take() {
            handle.abort();
        }

        info!("Heartbeat manager stopped");
    }

    /// Update heartbeat interval
    pub fn set_interval(&self, interval_ms: u64) {
        *self.interval_ms.write() = interval_ms;
    }

    /// Record heartbeat ACK received
    pub fn record_ack(&self) {
        *self.last_ack.write() = Some(Instant::now());
    }

    /// Check if heartbeat is healthy
    pub fn is_healthy(&self) -> bool {
        if let Some(last) = *self.last_ack.read() {
            let interval = Duration::from_millis(*self.interval_ms.read() * 3);
            last.elapsed() < interval
        } else {
            false
        }
    }
}