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
