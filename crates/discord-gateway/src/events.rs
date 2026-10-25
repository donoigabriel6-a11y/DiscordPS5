//! Gateway Event Types

use serde::{Deserialize, Serialize};
use serde_json::Value;

use discord_common::discord_types::*;

/// Gateway payload structure
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GatewayPayload {
    pub op: OpCode,
    pub d: Value,
    pub s: Option<u64>,
    pub t: Option<String>,
}

/// Message delete event
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MessageDelete {
    pub id: String,
    pub channel_id: String,
    pub guild_id: Option<String>,
}

/// Message delete bulk event
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MessageDeleteBulk {
    pub ids: Vec<String>,
    pub channel_id: String,
    pub guild_id: Option<String>,
}

/// Channel pins update
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChannelPinsUpdate {
    pub channel_id: String,
    pub guild_id: Option<String>,
    pub last_pin_timestamp: Option<String>,
}

/// Guild member update
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GuildMemberUpdate {
    pub guild_id: String,
    pub roles: Vec<String>,
    pub user: User,
    pub nick: Option<String>,
    pub avatar: Option<String>,
    pub joined_at: String,
    pub premium_since: Option<String>,
    pub deaf: bool,
    pub mute: bool,
    pub pending: bool,
    pub permissions: Option<String>,
    pub communication_disabled_until: Option<String>,
}

/// Guild member remove
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GuildMemberRemove {
    pub guild_id: String,
    pub user: User,
}

/// Guild members chunk
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GuildMembersChunk {
    pub guild_id: String,
    pub members: Vec<GuildMember>,
    pub chunk_index: u32,
    pub chunk_count: u32,
    pub not_found: Option<Vec<String>>,
    pub presences: Option<Vec<PresenceUpdate>>,
    pub nonce: Option<String>,
}

/// Role delete
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RoleDelete {
    pub guild_id: String,
    pub role_id: String,
}

/// Ban add
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BanAdd {
    pub guild_id: String,
    pub user: User,
}

/// Ban remove
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BanRemove {
    pub guild_id: String,
    pub user: User,
}

/// Guild emojis update
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GuildEmojisUpdate {
    pub guild_id: String,
    pub emojis: Vec<Emoji>,
}

/// Guild stickers update
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GuildStickersUpdate {
    pub guild_id: String,
    pub stickers: Vec<Sticker>,
}

/// Guild integrations update
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GuildIntegrationsUpdate {
    pub guild_id: String,
}

/// Guild webhooks update
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GuildWebhooksUpdate {
    pub guild_id: String,
    pub channel_id: String,
}

/// Invite
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Invite {
    pub code: String,
    pub guild_id: Option<String>,
    pub channel_id: String,
    pub inviter: Option<User>,
    pub target_user: Option<User>,
    pub target_type: Option<u8>,
    pub approximate_presence_count: Option<u32>,
    pub approximate_member_count: Option<u32>,
    pub expires_at: Option<String>,
}

/// Invite delete
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InviteDelete {
    pub channel_id: String,
    pub guild_id: Option<String>,
    pub code: String,
}

/// Thread list sync
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ThreadListSync {
    pub guild_id: String,
    pub channel_ids: Option<Vec<String>>,
    pub threads: Vec<Channel>,
    pub members: Vec<ThreadMember>,
}

/// Thread member update
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ThreadMemberUpdate {
    pub id: String,
    pub user_id: Option<String>,
    pub guild_id: String,
    pub member: ThreadMember,
}

/// Thread members update
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ThreadMembersUpdate {
    pub id: String,
    pub guild_id: String,
    pub member_count: u32,
    pub added_members: Option<Vec<ThreadMember>>,
    pub removed_member_ids: Option<Vec<String>>,
}

/// Presence update
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PresenceUpdate {
    pub user: User,
    pub guild_id: String,
    pub status: String,
    pub activities: Vec<Activity>,
    pub client_status: ClientStatus,
}

/// Client status
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ClientStatus {
    pub desktop: Option<String>,
    pub mobile: Option<String>,
    pub web: Option<String>,
}

/// Interaction
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Interaction {
    pub id: String,
    pub application_id: String,
    pub interaction_type: InteractionType,
    pub data: Option<InteractionData>,
    pub guild_id: Option<String>,
    pub channel_id: Option<String>,
    pub channel: Option<Channel>,
    pub member: Option<GuildMember>,
    pub user: Option<User>,
    pub token: String,
    pub version: u32,
    pub message: Option<Message>,
    pub app_permissions: Option<String>,
    pub locale: String,
    pub guild_locale: String,
    pub entitlements: Option<Vec<Entitlement>>,
    pub authorizing_integration_owners: Vec<String>,
    pub context: Option<InteractionContext>,
}

/// Interaction data
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InteractionData {
    pub id: String,
    pub name: String,
    pub interaction_type: InteractionType,
    pub resolved: Option<ResolvedData>,
    pub options: Option<Vec<InteractionDataOption>>,
    pub guild_id: Option<String>,
    pub target_id: Option<String>,
}

/// Interaction data option
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InteractionDataOption {
    pub name: String,
    pub interaction_type: InteractionType,
    pub value: Option<Value>,
    pub options: Option<Vec<InteractionDataOption>>,
    pub focused: Option<bool>,
}

/// Entitlement
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Entitlement {
    pub id: String,
    pub sku_id: String,
    pub application_id: String,
    pub user_id: String,
    pub type: EntitlementType,
    pub deleted: bool,
    pub starts_at: Option<String>,
    pub ends_at: Option<String>,
    pub guild_id: Option<String>,
    pub consumed: bool,
}

/// Entitlement type
#[derive(Debug, Clone, Copy, Serialize_repr, Deserialize_repr, PartialEq, Eq)]
#[repr(u8)]
pub enum EntitlementType {
    Purchase = 1,
    PremiumSubscription = 2,
    DeveloperGift = 3,
    TestModePurchase = 4,
    FreePurchase = 5,
    UserGift = 6,
    PremiumPurchase = 7,
}

/// Interaction context
#[derive(Debug, Clone, Copy, Serialize_repr, Deserialize_repr, PartialEq, Eq)]
#[repr(u8)]
pub enum InteractionContext {
    Guild = 0,
    BotDm = 1,
    PrivateChannel = 2,
}

/// Application command
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ApplicationCommand {
    pub id: String,
    pub application_id: String,
    pub version: String,
    pub default_member_permissions: Option<String>,
    pub type: ApplicationCommandType,
    pub name: String,
    pub description: String,
    pub description_localizations: Option<HashMap<String, String>>,
    pub name_localizations: Option<HashMap<String, String>>,
    pub options: Option<Vec<ApplicationCommandOption>>,
    pub dm_permission: Option<bool>,
    pub default_permission: Option<bool>,
    pub nsfw: Option<bool>,
    pub integration_types: Option<Vec<ApplicationIntegrationType>>,
    pub contexts: Option<Vec<InteractionContext>>,
}

/// Application command type
#[derive(Debug, Clone, Copy, Serialize_repr, Deserialize_repr, PartialEq, Eq)]
#[repr(u8)]
pub enum ApplicationCommandType {
    ChatInput = 1,
    User = 2,
    Message = 3,
}

/// Application command option
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ApplicationCommandOption {
    pub type: ApplicationCommandOptionType,
    pub name: String,
    pub description: String,
    pub name_localizations: Option<HashMap<String, String>>,
    pub description_localizations: Option<HashMap<String, String>>,
    pub required: Option<bool>,
    pub choices: Option<Vec<ApplicationCommandOptionChoice>>,
    pub options: Option<Vec<ApplicationCommandOption>>,
    pub channel_types: Option<Vec<ChannelType>>,
    pub min_value: Option<Value>,
    pub max_value: Option<Value>,
    pub min_length: Option<u32>,
    pub max_length: Option<u32>,
    pub autocomplete: Option<bool>,
}

/// Application command option type
#[derive(Debug, Clone, Copy, Serialize_repr, Deserialize_repr, PartialEq, Eq)]
#[repr(u8)]
pub enum ApplicationCommandOptionType {
    SubCommand = 1,
    SubCommandGroup = 2,
    String = 3,
    Integer = 4,
    Boolean = 5,
    User = 6,
    Channel = 7,
    Role = 8,
    Mentionable = 9,
    Number = 10,
    Attachment = 11,
}

/// Application command option choice
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ApplicationCommandOptionChoice {
    pub name: String,
    pub name_localizations: Option<HashMap<String, String>>,
    pub value: Value,
}

/// Application integration type
#[derive(Debug, Clone, Copy, Serialize_repr, Deserialize_repr, PartialEq, Eq)]
#[repr(u8)]
pub enum ApplicationIntegrationType {
    GuildInstall = 0,
    UserInstall = 1,
}

/// Application command delete
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ApplicationCommandDelete {
    pub id: String,
    pub application_id: String,
    pub version: String,
    pub guild_id: Option<String>,
    pub type: ApplicationCommandType,
    pub name: String,
}