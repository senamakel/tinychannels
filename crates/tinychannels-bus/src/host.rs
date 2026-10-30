//! Portable payloads for the host callback object.
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

pub use crate::voice::{HostTranscriptionRequest, HostTranscriptionResult};

/// An event published by a provider through the host's event sink.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
pub struct HostEventPublication {
    /// Event domain, for example `channel` or `web`.
    pub domain: String,
    /// Event kind within the domain.
    pub kind: String,
    /// Domain-specific JSON payload.
    pub payload: serde_json::Value,
}

/// Request for the host to persist voice media received by a channel module.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct HostStoreVoiceMediaRequest {
    pub scope: crate::HostVoiceScope,
    pub content_type: String,
    pub bytes: Vec<u8>,
    pub codec: Option<String>,
    pub duration_ms: Option<u64>,
}

/// Resulting durable media reference returned to the channel module.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct HostStoreVoiceMediaResult {
    pub media: crate::MediaReference,
}
