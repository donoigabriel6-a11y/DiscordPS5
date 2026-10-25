//! Discord REST API Endpoints and Parameters

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

use discord_common::discord_types::*;

/// Parameters for getting messages
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct GetMessagesParams {
    pub around: Option<String>,
    pub before: Option<String>,
    pub after: Option<String>,
    pub limit: Option<u32>,
}

impl GetMessagesParams {
    pub fn to_query(&self) -> Option<String> {
        let mut params = Vec::new();
        if let Some(v) = &self.around { params.push(format!("around={}", v)); }
        if let Some(v) = &self.before { params.push(format!("before={}", v)); }
        if let Some(v) = &self.after { params.push(format!("after={}", v)); }
        if let Some(v) = self.limit { params.push(format!("limit={}", v)); }
        if params.is_empty() { None } else { Some(params.join("&")) }
    }
}

/// Parameters for creating a message
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct CreateMessageParams {
    pub content: Option<String>,
    pub nonce: Option<String>,
    pub tts: Option<bool>,
    pub embeds: Option<Vec<Embed>>,
    pub components: Option<Vec<Component>>,
    pub allowed_mentions: Option<AllowedMentions>,
    pub message_reference: Option<MessageReference>,
    pub sticker_ids: Option<Vec<String>>,
    pub flags: Option<u32>,
}

/// Allowed mentions
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct AllowedMentions {
    pub parse: Option<Vec<AllowedMentionType>>,
    pub roles: Option<Vec<String>>,
    pub users: Option<Vec<String>>,
    pub replied_user: Option<bool>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AllowedMentionType {
    Roles,
    Users,
    Everyone,
}

/// Message reference
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MessageReference {
    pub message_id: Option<String>,
    pub channel_id: String,
    pub guild_id: Option<String>,
    pub fail_if_not_exists: Option<bool>,
}

/// Parameters for modifying a message
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ModifyMessageParams {
    pub content: Option<String>,
    pub embeds: Option<Vec<Embed>>,
    pub components: Option<Vec<Component>>,
    pub allowed_mentions: Option<AllowedMentions>,
    pub flags: Option<u32>,
}

/// Parameters for bulk deleting messages
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BulkDeleteMessagesParams {
    pub messages: Vec<String>,
}

/// Parameters for modifying a channel
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ModifyChannelParams {
    pub name: Option<String>,
    pub channel_type: Option<ChannelType>,
    pub position: Option<u32>,
    pub topic: Option<String>,
    pub nsfw: Option<bool>,
    pub rate_limit_per_user: Option<u32>,
    pub bitrate: Option<u32>,
    pub user_limit: Option<u32>,
    pub permission_overwrites: Option<Vec<PermissionOverwrite>>,
    pub parent_id: Option<String>,
    pub rtc_region: Option<String>,
    pub video_quality_mode: Option<VideoQualityMode>,
    pub default_auto_archive_duration: Option<u32>,
    pub flags: Option<u32>,
}

/// Parameters for getting guild members
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct GetGuildMembersParams {
    pub limit: Option<u32>,
    pub after: Option<String>,
}

impl GetGuildMembersParams {
    pub fn to_query(&self) -> Option<String> {
        let mut params = Vec::new();
        if let Some(v) = self.limit { params.push(format!("limit={}", v)); }
        if let Some(v) = &self.after { params.push(format!("after={}", v)); }
        if params.is_empty() { None } else { Some(params.join("&")) }
    }
}

/// Parameters for getting user guilds
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct GetGuildsParams {
    pub before: Option<String>,
    pub after: Option<String>,
    pub limit: Option<u32>,
    pub with_counts: Option<bool>,
}

impl GetGuildsParams {
    pub fn to_query(&self) -> Option<String> {
        let mut params = Vec::new();
        if let Some(v) = &self.before { params.push(format!("before={}", v)); }
        if let Some(v) = &self.after { params.push(format!("after={}", v)); }
        if let Some(v) = self.limit { params.push(format!("limit={}", v)); }
        if let Some(v) = self.with_counts { params.push(format!("with_counts={}", v)); }
        if params.is_empty() { None } else { Some(params.join("&")) }
    }
}

/// Parameters for modifying current user
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ModifyUserParams {
    pub avatar: Option<String>,
    pub banner: Option<String>,
}

/// Parameters for creating a DM
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateDMParams {
    pub recipient_id: String,
}

/// Voice region
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VoiceRegion {
    pub id: String,
    pub name: String,
    pub vip: bool,
    pub optimal: bool,
    pub deprecated: bool,
    pub custom: bool,
}

/// Parameters for creating a webhook
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateWebhookParams {
    pub name: String,
    pub avatar: Option<String>,
    pub channel_id: Option<String>,
}

/// Webhook
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Webhook {
    pub id: String,
    pub guild_id: Option<String>,
    pub channel_id: String,
    pub user: Option<User>,
    pub name: String,
    pub avatar: Option<String>,
    pub token: Option<String>,
    pub application_id: Option<String>,
    pub source_guild: Option<PartialGuild>,
    pub source_channel: Option<PartialChannel>,
    pub url: Option<String>,
}

/// Partial guild
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PartialGuild {
    pub id: String,
    pub name: String,
    pub icon: Option<String>,
}

/// Partial channel
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PartialChannel {
    pub id: String,
    pub name: String,
    pub channel_type: ChannelType,
}

/// Search parameters
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct SearchParams {
    pub content: Option<String>,
    pub author_id: Option<String>,
    pub channel_id: Option<String>,
    pub min_id: Option<String>,
    pub max_id: Option<String>,
    pub has: Option<String>,
    pub pinned: Option<bool>,
    pub include_nsfw: Option<bool>,
    pub sort_by: Option<SearchSortBy>,
    pub sort_order: Option<SearchSortOrder>,
    pub offset: Option<u32>,
    pub limit: Option<u32>,
}

impl SearchParams {
    pub fn to_query(&self) -> Option<String> {
        let mut params = Vec::new();
        if let Some(v) = &self.content { params.push(format!("content={}", urlencoding::encode(v))); }
        if let Some(v) = &self.author_id { params.push(format!("author_id={}", v)); }
        if let Some(v) = &self.channel_id { params.push(format!("channel_id={}", v)); }
        if let Some(v) = &self.min_id { params.push(format!("min_id={}", v)); }
        if let Some(v) = &self.max_id { params.push(format!("max_id={}", v)); }
        if let Some(v) = &self.has { params.push(format!("has={}", v)); }
        if let Some(v) = self.pinned { params.push(format!("pinned={}", v)); }
        if let Some(v) = self.include_nsfw { params.push(format!("include_nsfw={}", v)); }
        if let Some(v) = &self.sort_by { params.push(format!("sort_by={}", v as u8)); }
        if let Some(v) = &self.sort_order { params.push(format!("sort_order={}", v as u8)); }
        if let Some(v) = self.offset { params.push(format!("offset={}", v)); }
        if let Some(v) = self.limit { params.push(format!("limit={}", v)); }
        if params.is_empty() { None } else { Some(params.join("&")) }
    }
}

/// Search sort by
#[derive(Debug, Clone, Copy, Serialize_repr, Deserialize_repr, PartialEq, Eq)]
#[repr(u8)]
pub enum SearchSortBy {
    Relevance = 0,
    Timestamp = 1,
}

/// Search sort order
#[derive(Debug, Clone, Copy, Serialize_repr, Deserialize_repr, PartialEq, Eq)]
#[repr(u8)]
pub enum SearchSortOrder {
    Asc = 0,
    Desc = 1,
}

/// Search result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SearchResult {
    pub total_results: u32,
    pub messages: Vec<Vec<Message>>,
}

/// Connection
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Connection {
    pub id: String,
    pub name: String,
    pub type: String,
    pub revoked: bool,
    pub integrations: Option<Vec<Integration>>,
    pub verified: bool,
    pub friend_sync: bool,
    pub show_activity: bool,
    pub visibility: ConnectionVisibility,
}

/// Integration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Integration {
    pub id: String,
    pub name: String,
    pub type: String,
    pub enabled: bool,
    pub syncing: bool,
    pub role_id: Option<String>,
    pub enable_emoticons: bool,
    pub expire_behavior: u8,
    pub expire_grace_period: u32,
    pub user: User,
    pub account: IntegrationAccount,
    pub synced_at: String,
    pub subscriber_count: u32,
    pub revoked: bool,
    pub application: Option<IntegrationApplication>,
}

/// Integration account
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IntegrationAccount {
    pub id: String,
    pub name: String,
}

/// Integration application
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IntegrationApplication {
    pub id: String,
    pub name: String,
    pub icon: Option<String>,
    pub description: String,
    pub bot: Option<User>,
}

/// Connection visibility
#[derive(Debug, Clone, Copy, Serialize_repr, Deserialize_repr, PartialEq, Eq)]
#[repr(u8)]
pub enum ConnectionVisibility {
    None = 0,
    Everyone = 1,
}