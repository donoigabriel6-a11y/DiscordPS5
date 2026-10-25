//! Session Resume Management

use std::sync::Arc;
use std::time::Duration;

use anyhow::{Context, Result};
use parking_lot::RwLock;
use serde::{Deserialize, Serialize};
use tokio::time::sleep;
use tracing::{debug, error, info, warn};

use crate::{GatewayConnection, DiscordError, Result as GatewayResult};
use discord_common::discord_types::*;

/// Session data for resume
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SessionData {
    pub session_id: String,
    pub sequence: u64,
    pub token: String,
    pub shard: Option<[u32; 2]>,
    pub timestamp: u64,
}

/// Resume manager for handling session persistence
pub struct ResumeManager {
    connection: Arc<GatewayConnection>,
    session_data: Arc<RwLock<Option<SessionData>>>,
    max_retry_attempts: u32,
    retry_delay: Duration,
    running: Arc<RwLock<bool>>,
}

impl ResumeManager {
    pub fn new(connection: Arc<GatewayConnection>) -> Self {
        Self {
            connection,
            session_data: Arc::new(RwLock::new(None)),
            max_retry_attempts: 5,
            retry_delay: Duration::from_secs(5),
            running: Arc::new(RwLock::new(false)),
        }
    }

    /// Save session data for resume
    pub fn save_session(&self, session_id: String, sequence: u64, token: String, shard: Option<[u32; 2]>) {
        let data = SessionData {
            session_id,
            sequence,
            token,
            shard,
            timestamp: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_secs(),
        };
        *self.session_data.write() = Some(data);
        info!("Session saved for resume");
    }

    /// Get saved session data
    pub fn get_session(&self) -> Option<SessionData> {
        self.session_data.read().clone()
    }

    /// Clear session data
    pub fn clear_session(&self) {
        *self.session_data.write() = None;
        info!("Session cleared");
    }

    /// Attempt to resume session
    pub async fn attempt_resume(&self) -> GatewayResult<bool> {
        let session = match self.get_session() {
            Some(s) => s,
            None => return Ok(false),
        };

        info!("Attempting to resume session: {}", session.session_id);

        for attempt in 1..=self.max_retry_attempts {
            *self.connection.state.write() = ConnectionState::Resuming;

            if let Err(e) = self.send_resume(&session).await {
                warn!("Resume attempt {} failed: {}", attempt, e);
                if attempt < self.max_retry_attempts {
                    sleep(self.retry_delay).await;
                    continue;
                }
                return Err(e);
            }

            // Wait for RESUMED event
            // In a real implementation, this would wait for the event
            // For now, we'll just return success
            info!("Resume attempt {} sent", attempt);
            return Ok(true);
        }

        Ok(false)
    }

    async fn send_resume(&self, session: &SessionData) -> GatewayResult<()> {
        let resume = ResumeData {
            token: session.token.clone(),
            session_id: session.session_id.clone(),
            seq: session.sequence,
        };

        let payload = GatewayPayload {
            op: OpCode::Resume,
            d: serde_json::to_value(resume)?,
            s: None,
            t: None,
        };

        let msg = Message::Text(serde_json::to_string(&payload)?);

        if let Some(ws) = self.connection.ws_stream.lock().unwrap().as_mut() {
            ws.send(msg).await?;
        }

        Ok(())
    }

    /// Handle invalid session response
    pub async fn handle_invalid_session(&self, resumable: bool) -> GatewayResult<()> {
        if resumable {
            warn!("Session invalid but resumable, will retry");
            // Session data is still valid, will retry on next connect
        } else {
            warn!("Session invalid and not resumable, clearing session");
            self.clear_session();
        }
        Ok(())
    }
}