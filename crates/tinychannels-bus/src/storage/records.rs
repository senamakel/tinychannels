//! Serialized record DTOs stored by TinyChannels implementations.
//!
//! These types describe the persisted document shapes. Conversion between
//! channel messages and these records belongs to the `tinychannels` crate.

use crate::channel::{DeliveryDurability, OutboundPayload};
use crate::module_delivery::DeliveryRecoveryState;
use serde::{Deserialize, Serialize};

/// A chat's binding to a host conversation thread. When it was last bound is
/// the record's own `updated_ms` metadata, not a field here.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ChatBinding {
    pub thread_id: String,
    pub sender_key: String,
    /// Title captured at `/new` time so `/status` can show it without listing
    /// every thread.
    #[serde(default)]
    pub title: Option<String>,
}

/// The outbound intent's persisted shape. The payload is already snake_case
/// on the wire, so only the five camelCase fields differ.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct StoredIntent {
    pub idempotency_key: String,
    pub channel_id: String,
    pub conversation_id: String,
    pub reply_to_id: Option<String>,
    pub thread_id: Option<String>,
    pub durability: DeliveryDurability,
    pub payload: OutboundPayload,
}

/// A pending or failed delivery as stored. The entry id is the intent's
/// idempotency key, so the body does not repeat it.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct StoredDelivery {
    pub intent: StoredIntent,
    pub enqueued_ms: i64,
    #[serde(default)]
    pub retry_count: u32,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub last_attempt_ms: Option<i64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub last_error: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub platform_send_started_ms: Option<i64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub recovery_state: Option<DeliveryRecoveryState>,
}

/// A terminal result as stored. Its lifetime is the record's `expires_ms`
/// metadata, not a body field. The id is retained because a long idempotency
/// key may have a hashed storage document id.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct StoredResult {
    pub id: String,
    pub intent_fingerprint: String,
    pub completed_ms: i64,
}
