//! Discord REST API Client Implementation

use std::sync::Arc;
use std::time::Duration;

use anyhow::{Context, Result};
use parking_lot::RwLock;
use reqwest::{Client, RequestBuilder, Response, StatusCode};
use tokio::time::sleep;
use tracing::{debug, error, info, warn};

use crate::{DiscordError, Result as RestResult};
use crate::rate_limit::{RateLimiter, RateLimitBucket};
use crate::auth::AuthManager;
use crate::endpoints::*;
use crate::models::*;

/// REST API client for Discord
pub struct RestClient {
    http: Client,
    base_url: String,
    auth: Arc<AuthManager>,
    rate_limiter: Arc<RateLimiter>,
    user_agent: String,
}

impl RestClient {
    /// Create a new REST client
    pub fn new(token: String, base_url: Option<String>) -> RestResult<Self> {
        let http = Client::builder()
            .timeout(Duration::from_secs(30))
            .build()
            .context("Failed to create HTTP client")?;

        let base_url = base_url.unwrap_or_else(|| "https://discord.com/api/v10".to_string());

        Ok(Self {
            http,
            base_url,
            auth: Arc::new(AuthManager::new(token)),
            rate_limiter: Arc::new(RateLimiter::new()),
            user_agent: "DiscordPS5 (https://github.com/donoigabriel6-a11y/DiscordPS5, 0.1.0)".to_string(),
        })
    }

    /// Get the auth manager
    pub fn auth(&self) -> Arc<AuthManager> {
        self.auth.clone()
    }

    /// Get the rate limiter
    pub fn rate_limiter(&self) -> Arc<RateLimiter> {
        self.rate_limiter.clone()
    }

    /// Build a request with authentication and headers
    fn build_request(&self, method: reqwest::Method, endpoint: &str) -> RequestBuilder {
        let url = format!("{}{}", self.base_url, endpoint);

        self.http.request(method, &url)
            .header("Authorization", self.auth.authorization_header())
            .header("User-Agent", &self.user_agent)
            .header("Content-Type", "application/json")
            .header("X-RateLimit-Precision", "millisecond")
    }

    /// Execute a request with rate limiting and retry logic
    async fn execute<T: for<'de> serde::Deserialize<'de>>(
        &self,
        mut request: RequestBuilder,
        bucket: &str,
    ) -> RestResult<T> {
        // Wait for rate limit
        self.rate_limiter.acquire(bucket).await?;

        let response = request.send().await.context("Failed to send request")?;

        // Update rate limit info from headers
        self.update_rate_limit(bucket, &response);

        let status = response.status();

        if status.is_success() {
            let data = response.json().await.context("Failed to parse response")?;
            Ok(data)
        } else {
            self.handle_error(status, response).await
        }
    }

    /// Update rate limit from response headers
    fn update_rate_limit(&self, bucket: &str, response: &Response) {
        if let Some(limit) = response.headers().get("X-RateLimit-Limit") {
            if let Ok(limit) = limit.to_str().unwrap_or("").parse::<u32>() {
                let remaining = response.headers()
                    .get("X-RateLimit-Remaining")
                    .and_then(|h| h.to_str().ok())
                    .and_then(|s| s.parse::<u32>().ok())
                    .unwrap_or(limit);

                let reset_after = response.headers()
                    .get("X-RateLimit-Reset-After")
                    .and_then(|h| h.to_str().ok())
                    .and_then(|s| s.parse::<f64>().ok())
                    .unwrap_or(0.0);

                self.rate_limiter.update_bucket(bucket, limit, remaining, reset_after);
            }
        }

        // Handle global rate limit
        if let Some(retry_after) = response.headers().get("Retry-After") {
            if let Ok(retry_after) = retry_after.to_str().unwrap_or("").parse::<f64>() {
                self.rate_limiter.set_global_limit(retry_after);
            }
        }
    }

    /// Handle error responses
    async fn handle_error<T>(&self, status: StatusCode, response: Response) -> RestResult<T> {
        let body = response.text().await.unwrap_or_default();

        match status {
            StatusCode::TOO_MANY_REQUESTS => {
                // Rate limited
                let retry_after = body.parse::<f64>().unwrap_or(1.0);
                Err(DiscordError::RateLimited {
                    retry_after: retry_after.ceil() as u64
                }.into())
            }
            StatusCode::UNAUTHORIZED => {
                Err(DiscordError::InvalidToken.into())
            }
            StatusCode::FORBIDDEN => {
                Err(DiscordError::Http(reqwest::Error::from(body)).into())
            }
            StatusCode::NOT_FOUND => {
                Err(DiscordError::Http(reqwest::Error::from(body)).into())
            }
            StatusCode::INTERNAL_SERVER_ERROR | StatusCode::BAD_GATEWAY | StatusCode::SERVICE_UNAVAILABLE => {
                // Server error - retryable
                Err(DiscordError::Http(reqwest::Error::from(body)).into())
            }
            _ => {
                Err(DiscordError::Http(reqwest::Error::from(body)).into())
            }
        }
    }

    // ===== Channel Endpoints =====

    /// Get a channel
    pub async fn get_channel(&self, channel_id: &str) -> RestResult<Channel> {
        self.execute(
            self.build_request(reqwest::Method::GET, &format!("/channels/{}", channel_id)),
            "channels/:id",
        ).await
    }

    /// Modify a channel
    pub async fn modify_channel(&self, channel_id: &str, params: ModifyChannelParams) -> RestResult<Channel> {
        self.execute(
            self.build_request(reqwest::Method::PATCH, &format!("/channels/{}", channel_id))
                .json(&params),
            "channels/:id",
        ).await
    }

    /// Delete/close a channel
    pub async fn delete_channel(&self, channel_id: &str) -> RestResult<Channel> {
        self.execute(
            self.build_request(reqwest::Method::DELETE, &format!("/channels/{}", channel_id)),
            "channels/:id",
        ).await
    }

    /// Get channel messages
    pub async fn get_messages(&self, channel_id: &str, params: GetMessagesParams) -> RestResult<Vec<Message>> {
        let mut url = format!("/channels/{}/messages", channel_id);
        if let Some(query) = params.to_query() {
            url.push('?');
            url.push_str(&query);
        }

        self.execute(
            self.build_request(reqwest::Method::GET, &url),
            "channels/:id/messages",
        ).await
    }

    /// Get a message
    pub async fn get_message(&self, channel_id: &str, message_id: &str) -> RestResult<Message> {
        self.execute(
            self.build_request(reqwest::Method::GET, &format!("/channels/{}/messages/{}", channel_id, message_id)),
            "channels/:id/messages/:id",
        ).await
    }

    /// Create a message
    pub async fn create_message(&self, channel_id: &str, params: CreateMessageParams) -> RestResult<Message> {
        self.execute(
            self.build_request(reqwest::Method::POST, &format!("/channels/{}/messages", channel_id))
                .json(&params),
            "channels/:id/messages",
        ).await
    }

    /// Create a message with file upload
    pub async fn create_message_with_files(&self, channel_id: &str, params: CreateMessageParams, files: Vec<(&str, Vec<u8>)>) -> RestResult<Message> {
        // For multipart uploads, we need a different approach
        // This is a simplified version
        self.create_message(channel_id, params).await
    }

    /// Crosspost a message (announcement channels)
    pub async fn crosspost_message(&self, channel_id: &str, message_id: &str) -> RestResult<Message> {
        self.execute(
            self.build_request(reqwest::Method::POST, &format!("/channels/{}/messages/{}/crosspost", channel_id, message_id)),
            "channels/:id/messages/:id/crosspost",
        ).await
    }

    /// Modify a message
    pub async fn modify_message(&self, channel_id: &str, message_id: &str, params: ModifyMessageParams) -> RestResult<Message> {
        self.execute(
            self.build_request(reqwest::Method::PATCH, &format!("/channels/{}/messages/{}", channel_id, message_id))
                .json(&params),
            "channels/:id/messages/:id",
        ).await
    }

    /// Delete a message
    pub async fn delete_message(&self, channel_id: &str, message_id: &str) -> RestResult<()> {
        self.execute::<()>(
            self.build_request(reqwest::Method::DELETE, &format!("/channels/{}/messages/{}", channel_id, message_id)),
            "channels/:id/messages/:id",
        ).await
    }

    /// Bulk delete messages
    pub async fn bulk_delete_messages(&self, channel_id: &str, params: BulkDeleteMessagesParams) -> RestResult<()> {
        self.execute::<()>(
            self.build_request(reqwest::Method::POST, &format!("/channels/{}/messages/bulk-delete", channel_id))
                .json(&params),
            "channels/:id/messages/bulk-delete",
        ).await
    }

    // ===== Guild Endpoints =====

    /// Get a guild
    pub async fn get_guild(&self, guild_id: &str, with_counts: bool) -> RestResult<Guild> {
        let mut url = format!("/guilds/{}", guild_id);
        if with_counts {
            url.push_str("?with_counts=true");
        }

        self.execute(
            self.build_request(reqwest::Method::GET, &url),
            "guilds/:id",
        ).await
    }

    /// Get guild channels
    pub async fn get_guild_channels(&self, guild_id: &str) -> RestResult<Vec<Channel>> {
        self.execute(
            self.build_request(reqwest::Method::GET, &format!("/guilds/{}/channels", guild_id)),
            "guilds/:id/channels",
        ).await
    }

    /// Get guild members
    pub async fn get_guild_members(&self, guild_id: &str, params: GetGuildMembersParams) -> RestResult<Vec<GuildMember>> {
        let mut url = format!("/guilds/{}/members", guild_id);
        if let Some(query) = params.to_query() {
            url.push('?');
            url.push_str(&query);
        }

        self.execute(
            self.build_request(reqwest::Method::GET, &url),
            "guilds/:id/members",
        ).await
    }

    /// Get guild member
    pub async fn get_guild_member(&self, guild_id: &str, user_id: &str) -> RestResult<GuildMember> {
        self.execute(
            self.build_request(reqwest::Method::GET, &format!("/guilds/{}/members/{}", guild_id, user_id)),
            "guilds/:id/members/:id",
        ).await
    }

    /// Get current user guilds
    pub async fn get_current_user_guilds(&self, params: GetGuildsParams) -> RestResult<Vec<Guild>> {
        let mut url = "/users/@me/guilds".to_string();
        if let Some(query) = params.to_query() {
            url.push('?');
            url.push_str(&query);
        }

        self.execute(
            self.build_request(reqwest::Method::GET, &url),
            "users/@me/guilds",
        ).await
    }

    // ===== User Endpoints =====

    /// Get current user
    pub async fn get_current_user(&self) -> RestResult<User> {
        self.execute(
            self.build_request(reqwest::Method::GET, "/users/@me"),
            "users/@me",
        ).await
    }

    /// Get user
    pub async fn get_user(&self, user_id: &str) -> RestResult<User> {
        self.execute(
            self.build_request(reqwest::Method::GET, &format!("/users/{}", user_id)),
            "users/:id",
        ).await
    }

    /// Modify current user
    pub async fn modify_current_user(&self, params: ModifyUserParams) -> RestResult<User> {
        self.execute(
            self.build_request(reqwest::Method::PATCH, "/users/@me")
                .json(&params),
            "users/@me",
        ).await
    }

    /// Get user connections
    pub async fn get_user_connections(&self) -> RestResult<Vec<Connection>> {
        self.execute(
            self.build_request(reqwest::Method::GET, "/users/@me/connections"),
            "users/@me/connections",
        ).await
    }

    // ===== DM Endpoints =====

    /// Create DM
    pub async fn create_dm(&self, params: CreateDMParams) -> RestResult<Channel> {
        self.execute(
            self.build_request(reqwest::Method::POST, "/users/@me/channels")
                .json(&params),
            "users/@me/channels",
        ).await
    }

    // ===== Voice Endpoints =====

    /// Get voice regions
    pub async fn get_voice_regions(&self) -> RestResult<Vec<VoiceRegion>> {
        self.execute(
            self.build_request(reqwest::Method::GET, "/voice/regions"),
            "voice/regions",
        ).await
    }

    // ===== Invite Endpoints =====

    /// Get invite
    pub async fn get_invite(&self, code: &str, with_counts: bool, with_expiration: bool) -> RestResult<Invite> {
        let mut url = format!("/invites/{}", code);
        let mut params = Vec::new();
        if with_counts { params.push("with_counts=true"); }
        if with_expiration { params.push("with_expiration=true"); }
        if !params.is_empty() {
            url.push('?');
            url.push_str(&params.join("&"));
        }

        self.execute(
            self.build_request(reqwest::Method::GET, &url),
            "invites/:code",
        ).await
    }

    /// Delete invite
    pub async fn delete_invite(&self, code: &str) -> RestResult<Invite> {
        self.execute(
            self.build_request(reqwest::Method::DELETE, &format!("/invites/{}", code)),
            "invites/:code",
        ).await
    }

    // ===== Webhook Endpoints =====

    /// Create webhook
    pub async fn create_webhook(&self, channel_id: &str, params: CreateWebhookParams) -> RestResult<Webhook> {
        self.execute(
            self.build_request(reqwest::Method::POST, &format!("/channels/{}/webhooks", channel_id))
                .json(&params),
            "channels/:id/webhooks",
        ).await
    }

    // ===== Search =====

    /// Search guild messages
    pub async fn search_guild_messages(&self, guild_id: &str, params: SearchParams) -> RestResult<SearchResult> {
        let mut url = format!("/guilds/{}/messages/search", guild_id);
        if let Some(query) = params.to_query() {
            url.push('?');
            url.push_str(&query);
        }

        self.execute(
            self.build_request(reqwest::Method::GET, &url),
            "guilds/:id/messages/search",
        ).await
    }
}