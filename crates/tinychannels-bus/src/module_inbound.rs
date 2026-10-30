//! Inbound projections prepared by the module for host persistence and dispatch.
use crate::channel::SessionKeyPolicy;
use crate::{ChannelInboundEnvelope, ChannelMessage};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

/// Original provider data, preserving the normalized envelope when available.
#[derive(Clone, Serialize, Deserialize, JsonSchema)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum ChannelInboundSource {
    Legacy {
        message: ChannelMessage,
        #[serde(default)]
        envelope: Option<ChannelInboundEnvelope>,
    },
    Envelope {
        envelope: ChannelInboundEnvelope,
        timestamp: u64,
    },
}

/// Host-selected namespace and session isolation applied to provider data.
#[derive(Clone, Serialize, Deserialize, JsonSchema)]
pub struct PrepareChannelInboundRequest {
    pub source: ChannelInboundSource,
    pub namespace: String,
    pub session_policy: SessionKeyPolicy,
    #[serde(default)]
    pub preparation_mode: ChannelInboundPreparationMode,
}

/// Selects the stable typed projection or the one-release legacy message projection.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum ChannelInboundPreparationMode {
    #[default]
    Typed,
    Legacy,
    /// Compatibility spelling used by the host adapter for legacy text-only
    /// channels. It has the same projection semantics as `Legacy`.
    LegacyText,
}

/// Facts used by the host without repeating channel projection or key algorithms.
/// Authenticated transport provenance must be retained separately by the caller;
/// the envelope deliberately does not serialize its relay trust flag.
#[derive(Clone, Serialize, Deserialize, JsonSchema)]
pub struct PreparedChannelInbound {
    pub message: ChannelMessage,
    pub envelope: ChannelInboundEnvelope,
    pub history_key: String,
    pub thread_id: String,
    pub client_id: String,
    pub session_key: String,
}

/// Optional sender facts from hosted ingress, including legacy channel-only traffic.
#[derive(Clone, Serialize, Deserialize, JsonSchema)]
pub struct PrepareChannelIdentityRequest {
    pub channel: String,
    pub sender: Option<String>,
    pub reply_target: Option<String>,
    pub thread_ts: Option<String>,
}

/// Stable persisted identifiers; hosts must use these values verbatim.
#[derive(Clone, Serialize, Deserialize, JsonSchema)]
pub struct PreparedChannelIdentity {
    pub thread_id: String,
    pub client_id: String,
}

#[cfg(test)]
#[path = "module_inbound_tests.rs"]
mod tests;
