//! Discord REST API Response Models

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

use discord_common::discord_types::*;

/// Re-export common types
pub use discord_common::discord_types::*;

/// Gateway bot info
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GatewayBotInfo {
    pub url: String,
    pub shards: u32,
    pub session_start_limit: SessionStartLimit,
    pub shard_count: u32,
}

/// Session start limit
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SessionStartLimit {
    pub total: u32,
    pub remaining: u32,
    pub reset_after: u32,
    pub max_concurrency: u32,
}

/// OAuth2 token response
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TokenResponse {
    pub access_token: String,
    pub token_type: String,
    pub expires_in: u32,
    pub refresh_token: String,
    pub scope: String,
}

/// Application info
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ApplicationInfo {
    pub id: String,
    pub name: String,
    pub icon: Option<String>,
    pub description: String,
    pub rpc_origins: Option<Vec<String>>,
    pub bot_public: bool,
    pub bot_require_code_grant: bool,
    pub terms_of_service_url: Option<String>,
    pub privacy_policy_url: Option<String>,
    pub owner: Option<User>,
    pub summary: String,
    pub verify_key: String,
    pub team: Option<Team>,
    pub guild_id: Option<String>,
    pub primary_sku_id: Option<String>,
    pub slug: Option<String>,
    pub cover_image: Option<String>,
    pub flags: u32,
    pub tags: Vec<String>,
    pub install_params: Option<InstallParams>,
    pub custom_install_url: Option<String>,
}

/// Team
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Team {
    pub id: String,
    pub name: String,
    pub icon: Option<String>,
    pub owner_user_id: String,
    pub members: Vec<TeamMember>,
}

/// Team member
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TeamMember {
    pub membership_state: TeamMembershipState,
    pub permissions: Vec<String>,
    pub team_id: String,
    pub user: User,
}

/// Team membership state
#[derive(Debug, Clone, Copy, Serialize_repr, Deserialize_repr, PartialEq, Eq)]
#[repr(u8)]
pub enum TeamMembershipState {
    Invited = 1,
    Accepted = 2,
}

/// Guild template
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GuildTemplate {
    pub code: String,
    pub name: String,
    pub description: Option<String>,
    pub usage_count: u32,
    pub creator: User,
    pub created_at: String,
    pub updated_at: String,
    pub source_guild_id: String,
    pub serialized_source_guild: Guild,
    pub is_dirty: Option<bool>,
}

/// Guild widget
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GuildWidget {
    pub id: String,
    pub name: String,
    pub instant_invite: Option<String>,
    pub channels: Vec<GuildWidgetChannel>,
    pub members: Vec<GuildWidgetMember>,
    pub presence_count: u32,
}

/// Guild widget channel
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GuildWidgetChannel {
    pub id: String,
    pub name: String,
    pub position: u32,
    pub channel_type: ChannelType,
}

/// Guild widget member
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GuildWidgetMember {
    pub id: String,
    pub username: String,
    pub discriminator: String,
    pub avatar: Option<String>,
    pub status: String,
    pub avatar_url: String,
}

/// Welcome screen
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WelcomeScreen {
    pub description: Option<String>,
    pub welcome_channels: Vec<WelcomeScreenChannel>,
}

/// Welcome screen channel
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WelcomeScreenChannel {
    pub channel_id: String,
    pub description: String,
    pub emoji_id: Option<String>,
    pub emoji_name: Option<String>,
}

/// Ban
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Ban {
    pub reason: Option<String>,
    pub user: User,
}

/// Audit log
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuditLog {
    pub webhooks: Vec<Webhook>,
    pub users: Vec<User>,
    pub audit_log_entries: Vec<AuditLogEntry>,
    pub integrations: Vec<Integration>,
    pub threads: Vec<Channel>,
    pub auto_moderation_rules: Vec<AutoModerationRule>,
}

/// Audit log entry
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuditLogEntry {
    pub target_id: Option<String>,
    pub changes: Option<Vec<AuditLogChange>>,
    pub user_id: Option<String>,
    pub id: String,
    pub action_type: AuditLogEvent,
    pub options: Option<AuditLogEntryOptions>,
    pub reason: Option<String>,
}

/// Audit log change
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuditLogChange {
    pub new_value: Option<serde_json::Value>,
    pub old_value: Option<serde_json::Value>,
    pub key: String,
}

/// Audit log entry options
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuditLogEntryOptions {
    pub delete_member_days: Option<String>,
    pub members_removed: Option<String>,
    pub channel_id: Option<String>,
    pub count: Option<String>,
    pub id: Option<String>,
    pub type: Option<String>,
    pub role_name: Option<String>,
}

/// Audit log event
#[derive(Debug, Clone, Copy, Serialize_repr, Deserialize_repr, PartialEq, Eq)]
#[repr(u8)]
pub enum AuditLogEvent {
    GuildUpdate = 1,
    ChannelCreate = 10,
    ChannelUpdate = 11,
    ChannelDelete = 12,
    ChannelOverwriteCreate = 13,
    ChannelOverwriteUpdate = 14,
    ChannelOverwriteDelete = 15,
    MemberKick = 20,
    MemberPrune = 21,
    MemberBanAdd = 22,
    MemberBanRemove = 23,
    MemberUpdate = 24,
    MemberRoleUpdate = 25,
    MemberMove = 26,
    MemberDisconnect = 27,
    BotAdd = 28,
    RoleCreate = 30,
    RoleUpdate = 31,
    RoleDelete = 32,
    InviteCreate = 40,
    InviteUpdate = 41,
    InviteDelete = 42,
    WebhookCreate = 50,
    WebhookUpdate = 51,
    WebhookDelete = 52,
    EmojiCreate = 60,
    EmojiUpdate = 61,
    EmojiDelete = 62,
    MessageDelete = 72,
    MessageBulkDelete = 73,
    MessagePin = 74,
    MessageUnpin = 75,
    IntegrationCreate = 80,
    IntegrationUpdate = 81,
    IntegrationDelete = 82,
    StageInstanceCreate = 83,
    StageInstanceUpdate = 84,
    StageInstanceDelete = 85,
    StickerCreate = 90,
    StickerUpdate = 91,
    StickerDelete = 92,
    GuildScheduledEventCreate = 100,
    GuildScheduledEventUpdate = 101,
    GuildScheduledEventDelete = 102,
    ThreadCreate = 110,
    ThreadUpdate = 111,
    ThreadDelete = 112,
    ApplicationCommandCreate = 121,
    ApplicationCommandUpdate = 122,
    ApplicationCommandDelete = 123,
    AutoModerationRuleCreate = 140,
    AutoModerationRuleUpdate = 141,
    AutoModerationRuleDelete = 142,
    AutoModerationBlockMessage = 143,
    AutoModerationFlagMessage = 144,
    AutoModerationTimeoutMember = 145,
    CreatorMonetizationTermsAccepted = 150,
    OnboardingPromptCreate = 160,
    OnboardingPromptUpdate = 161,
    OnboardingPromptDelete = 162,
    OnboardingCreate = 163,
    OnboardingUpdate = 164,
    OnboardingDelete = 165,
    HomeSettingsCreate = 190,
    HomeSettingsUpdate = 191,
}

/// Auto moderation rule
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AutoModerationRule {
    pub id: String,
    pub guild_id: String,
    pub name: String,
    pub creator_id: String,
    pub event_type: AutoModerationEventType,
    pub trigger_type: AutoModerationTriggerType,
    pub trigger_metadata: AutoModerationTriggerMetadata,
    pub actions: Vec<AutoModerationAction>,
    pub enabled: bool,
    pub exempt_roles: Vec<String>,
    pub exempt_channels: Vec<String>,
}

/// Auto moderation event type
#[derive(Debug, Clone, Copy, Serialize_repr, Deserialize_repr, PartialEq, Eq)]
#[repr(u8)]
pub enum AutoModerationEventType {
    MessageSend = 1,
}

/// Auto moderation trigger type
#[derive(Debug, Clone, Copy, Serialize_repr, Deserialize_repr, PartialEq, Eq)]
#[repr(u8)]
pub enum AutoModerationTriggerType {
    Keyword = 1,
    Spam = 3,
    KeywordPreset = 4,
    MentionSpam = 5,
}

/// Auto moderation trigger metadata
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AutoModerationTriggerMetadata {
    pub keyword_filter: Option<Vec<String>>,
    pub regex_patterns: Option<Vec<String>>,
    pub presets: Option<Vec<AutoModerationKeywordPreset>>,
    pub allow_list: Option<Vec<String>>,
    pub mention_total_limit: Option<u32>,
}

/// Auto moderation keyword preset
#[derive(Debug, Clone, Copy, Serialize_repr, Deserialize_repr, PartialEq, Eq)]
#[repr(u8)]
pub enum AutoModerationKeywordPreset {
    Profanity = 1,
    SexualContent = 2,
    Slurs = 3,
}

/// Auto moderation action
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AutoModerationAction {
    pub type: AutoModerationActionType,
    pub metadata: Option<AutoModerationActionMetadata>,
}

/// Auto moderation action type
#[derive(Debug, Clone, Copy, Serialize_repr, Deserialize_repr, PartialEq, Eq)]
#[repr(u8)]
pub enum AutoModerationActionType {
    BlockMessage = 1,
    SendAlertMessage = 2,
    Timeout = 3,
    BlockMemberInteraction = 4,
}

/// Auto moderation action metadata
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AutoModerationActionMetadata {
    pub channel_id: Option<String>,
    pub duration_seconds: Option<u32>,
    pub custom_message: Option<String>,
}

/// Scheduled event
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScheduledEvent {
    pub id: String,
    pub guild_id: String,
    pub channel_id: Option<String>,
    pub creator_id: Option<String>,
    pub name: String,
    pub description: Option<String>,
    pub scheduled_start_time: String,
    pub scheduled_end_time: Option<String>,
    pub privacy_level: ScheduledEventPrivacyLevel,
    pub status: ScheduledEventStatus,
    pub entity_type: ScheduledEventEntityType,
    pub entity_id: Option<String>,
    pub entity_metadata: Option<ScheduledEventEntityMetadata>,
    pub creator: Option<User>,
    pub user_count: Option<u32>,
    pub image: Option<String>,
}

/// Scheduled event privacy level
#[derive(Debug, Clone, Copy, Serialize_repr, Deserialize_repr, PartialEq, Eq)]
#[repr(u8)]
pub enum ScheduledEventPrivacyLevel {
    GuildOnly = 2,
}

/// Scheduled event status
#[derive(Debug, Clone, Copy, Serialize_repr, Deserialize_repr, PartialEq, Eq)]
#[repr(u8)]
pub enum ScheduledEventStatus {
    Scheduled = 1,
    Active = 2,
    Completed = 3,
    Canceled = 4,
}

/// Scheduled event entity type
#[derive(Debug, Clone, Copy, Serialize_repr, Deserialize_repr, PartialEq, Eq)]
#[repr(u8)]
pub enum ScheduledEventEntityType {
    StageInstance = 1,
    Voice = 2,
    External = 3,
}

/// Scheduled event entity metadata
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScheduledEventEntityMetadata {
    pub location: Option<String>,
}

/// Stage instance
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StageInstance {
    pub id: String,
    pub guild_id: String,
    pub channel_id: String,
    pub topic: String,
    pub privacy_level: StageInstancePrivacyLevel,
    pub discoverable_disabled: bool,
    pub guild_scheduled_event_id: Option<String>,
}

/// Stage instance privacy level
#[derive(Debug, Clone, Copy, Serialize_repr, Deserialize_repr, PartialEq, Eq)]
#[repr(u8)]
pub enum StageInstancePrivacyLevel {
    Public = 1,
    GuildOnly = 2,
}

/// Thread member
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ThreadMember {
    pub id: Option<String>,
    pub user_id: Option<String>,
    pub join_timestamp: String,
    pub flags: u32,
    pub member: Option<GuildMember>,
}

/// Sticker pack
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StickerPack {
    pub id: String,
    pub stickers: Vec<Sticker>,
    pub name: String,
    pub sku_id: String,
    pub cover_sticker_id: Option<String>,
    pub description: String,
    pub banner_asset_id: Option<String>,
}

/// SKU
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SKU {
    pub id: String,
    pub type: SKUType,
    pub application_id: String,
    pub name: String,
    pub slug: String,
    pub flags: u32,
}

/// SKU type
#[derive(Debug, Clone, Copy, Serialize_repr, Deserialize_repr, PartialEq, Eq)]
#[repr(u8)]
pub enum SKUType {
    Durable = 2,
    Consumable = 3,
    Bundle = 5,
    Subscription = 6,
    SubscriptionGroup = 7,
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

/// Interaction response
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InteractionResponse {
    pub type: InteractionResponseType,
    pub data: Option<InteractionCallbackData>,
}

/// Interaction response type
#[derive(Debug, Clone, Copy, Serialize_repr, Deserialize_repr, PartialEq, Eq)]
#[repr(u8)]
pub enum InteractionResponseType {
    Pong = 1,
    ChannelMessageWithSource = 4,
    DeferredChannelMessageWithSource = 5,
    DeferredUpdateMessage = 6,
    UpdateMessage = 7,
    ApplicationCommandAutocompleteResult = 8,
    Modal = 9,
}

/// Interaction callback data
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InteractionCallbackData {
    pub tts: Option<bool>,
    pub content: Option<String>,
    pub embeds: Option<Vec<Embed>>,
    pub allowed_mentions: Option<AllowedMentions>,
    pub flags: Option<u32>,
    pub components: Option<Vec<Component>>,
    pub attachments: Option<Vec<Attachment>>,
    pub choices: Option<Vec<ApplicationCommandOptionChoice>>,
    pub custom_id: Option<String>,
    pub title: Option<String>,
    pub components: Option<Vec<Component>>,
}