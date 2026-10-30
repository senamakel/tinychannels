//! Outbound channel intent types.

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use serde_json::Value;

/// Delivery durability requested by core when sending agent output.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum DeliveryDurability {
    Required,
    #[default]
    BestEffort,
    Disabled,
}

/// Provider-neutral outbound payload variants.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum OutboundPayload {
    Text {
        text: String,
    },
    Media {
        text: Option<String>,
        media_urls: Vec<String>,
    },
    Voice {
        media_url: String,
    },
    Files {
        file_urls: Vec<String>,
    },
    Poll {
        question: String,
        options: Vec<String>,
    },
    PresentationBlocks {
        blocks: Value,
    },
    NativeChannelData {
        data: Value,
    },
}

/// Logical outbound send request.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct ChannelOutboundIntent {
    pub idempotency_key: String,
    pub channel_id: String,
    pub conversation_id: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub channel_ref: Option<super::types::ChannelRef>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub conversation_ref: Option<super::types::ConversationRef>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub turn_id: Option<String>,
    pub reply_to_id: Option<String>,
    pub thread_id: Option<String>,
    pub durability: DeliveryDurability,
    pub payload: OutboundPayload,
}

/// Provider-ready typed send assembled from a logical outbound request.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct ChannelOutboundMessage {
    pub channel_id: String,
    pub conversation_id: String,
    pub channel_ref: Option<super::types::ChannelRef>,
    pub thread_id: Option<String>,
    pub conversation_ref: Option<super::types::ConversationRef>,
    pub reply_to_id: Option<String>,
    pub turn_id: Option<String>,
    pub idempotency_key: String,
    pub parts: Vec<super::content::OutboundContentPart>,
}

impl ChannelOutboundMessage {
    pub fn from_legacy_message(channel_id: &str, message: &crate::SendMessage) -> Self {
        Self {
            channel_id: channel_id.to_owned(),
            conversation_id: message.recipient.clone(),
            channel_ref: None,
            thread_id: message.thread_ts.clone(),
            conversation_ref: None,
            reply_to_id: None,
            turn_id: None,
            idempotency_key: message.idempotency_key.clone().unwrap_or_default(),
            parts: vec![super::content::OutboundContentPart::Content {
                part: super::content::ContentPart::Text { text: message.content.clone(), markdown_dialect: None },
            }],
        }
    }
}

/// Result of sending one normalized outbound message.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(default)]
pub struct ChannelSendOutcome {
    pub receipt: super::receipt::MessageReceipt,
    pub error: Option<super::error::ChannelSendError>,
}

impl Default for ChannelOutboundIntent {
    fn default() -> Self {
        Self {
            idempotency_key: String::new(),
            channel_id: String::new(),
            conversation_id: String::new(),
            channel_ref: None,
            conversation_ref: None,
            turn_id: None,
            reply_to_id: None,
            thread_id: None,
            durability: DeliveryDurability::BestEffort,
            payload: OutboundPayload::Text {
                text: String::new(),
            },
        }
    }
}
