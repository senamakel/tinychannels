//! Portable provider event facts.
use super::types::{ChannelRef, ConversationRef};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

/// A normalized event emitted by a provider after message delivery.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct ChannelEvent {
    pub event_id: String,
    pub channel: ChannelRef,
    pub conversation: Option<ConversationRef>,
    pub event: ChannelEventKind,
    pub timestamp_unix_ms: Option<u64>,
}

/// Provider-neutral event payload.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum ChannelEventKind {
    ReadReceipt {
        message_id: String,
        read_at_unix_ms: Option<u64>,
    },
    ReactionAdded {
        message_id: String,
        reaction: ChannelReaction,
        actor_id: Option<String>,
    },
    ReactionRemoved {
        message_id: String,
        reaction: ChannelReaction,
        actor_id: Option<String>,
    },
    MessageEdited {
        message_id: String,
        text: String,
    },
    MessageDeleted {
        message_id: String,
    },
}

/// Normalized emoji or provider-native reaction identity.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct ChannelReaction {
    pub value: String,
    pub custom: bool,
}
