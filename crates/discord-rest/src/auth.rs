//! Authentication Management

use std::sync::Arc;
use parking_lot::RwLock;
use tracing::{debug, info};

use crate::{DiscordError, Result as RestResult};

/// Authentication manager for Discord API
pub struct AuthManager {
    token: RwLock<String>,
    token_type: RwLock<String>,
}

impl AuthManager {
    pub fn new(token: String) -> Self {
        Self {
            token: RwLock::new(token),
            token_type: RwLock::new("Bot".to_string()),
        }
    }

    pub fn set_token(&self, token: String) {
        *self.token.write() = token;
    }

    pub fn get_token(&self) -> String {
        self.token.read().clone()
    }

    pub fn set_token_type(&self, token_type: String) {
        *self.token_type.write() = token_type;
    }

    pub fn authorization_header(&self) -> String {
        format!("{} {}", *self.token_type.read(), *self.token.read())
    }

    pub fn is_bot(&self) -> bool {
        *self.token_type.read() == "Bot"
    }

    pub fn is_bearer(&self) -> bool {
        *self.token_type.read() == "Bearer"
    }
}