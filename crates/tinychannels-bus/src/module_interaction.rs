//! Provider interaction vocabulary for running module-owned channels.
use crate::{ChannelCapabilities, SendMessage};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

/// An interaction on one running provider. The module orders these with sends.
#[derive(Clone, Debug, Serialize, Deserialize, JsonSchema)]
pub struct ChannelInteractionRequest {
    pub channel: String,
    pub interaction: ChannelInteraction,
}

/// Provider operations used while composing a response.
#[derive(Clone, Debug, Serialize, Deserialize, JsonSchema)]
#[serde(tag = "operation", content = "request", rename_all = "snake_case")]
pub enum ChannelInteraction {
    Describe,
    StartTyping {
        recipient: String,
    },
    /// Refresh an active typing indicator inside the provider owner. The
    /// initial indicator is started by `StartTyping`; this keeps subsequent
    /// provider calls and their cancellation under the module lifecycle.
    StartTypingRefresh {
        recipient: String,
        interval_secs: u64,
    },
    StopTyping {
        recipient: String,
    },
    /// Stop and join the module-owned refresh task before acknowledging.
    StopTypingRefresh {
        handle: String,
    },
    SendDraft {
        message: SendMessage,
    },
    UpdateDraft {
        recipient: String,
        message_id: String,
        text: String,
    },
    FinalizeDraft {
        recipient: String,
        message_id: String,
        text: String,
        thread_ts: Option<String>,
    },
    EditMessage {
        receipt: crate::channel::MessageReceipt,
        intent: crate::channel::ChannelOutboundMessage,
    },
    DeleteMessage {
        receipt: crate::channel::MessageReceipt,
    },
}

/// Instance-specific facts supplied by the provider, rather than guessed by hosts.
#[derive(Clone, Debug, Serialize, Deserialize, JsonSchema)]
pub struct ChannelInteractionInfo {
    pub capabilities: ChannelCapabilities,
    pub proactive_target: Option<String>,
    pub supports_reactions: bool,
    pub supports_draft_updates: bool,
}

/// Extended configured-provider facts consumed by newer host adapters.
#[derive(Clone, Debug, Default, Serialize, Deserialize, JsonSchema)]
#[serde(default, rename_all = "camelCase")]
pub struct ChannelInteractionInfoV2 {
    pub capabilities: ChannelInteractionCapabilitiesV2,
}

#[derive(Clone, Debug, Default, Serialize, Deserialize, JsonSchema)]
#[serde(default, rename_all = "camelCase")]
pub struct ChannelInteractionCapabilitiesV2 {
    pub account: ChannelAccountCapabilities,
    pub reactions: bool,
    pub progressive_edits: bool,
}

#[derive(Clone, Debug, Default, Serialize, Deserialize, JsonSchema)]
#[serde(default, rename_all = "camelCase")]
pub struct ChannelAccountCapabilities {
    pub proactive_target: Option<String>,
}

/// Pre-v2 capability projection retained for older host interfaces.
#[derive(Clone, Copy, Debug, Default, Serialize, Deserialize, JsonSchema)]
pub struct LegacyChannelCapabilities {
    pub remote_control: bool,
    pub chat_approvals: bool,
    pub progressive_edits: bool,
    pub history_key_ignores_thread: bool,
}

impl From<&ChannelInteractionCapabilitiesV2> for LegacyChannelCapabilities {
    fn from(value: &ChannelInteractionCapabilitiesV2) -> Self {
        Self {
            progressive_edits: value.progressive_edits,
            ..Self::default()
        }
    }
}

/// Result of a provider interaction.
#[derive(Clone, Debug, Serialize, Deserialize, JsonSchema)]
#[serde(tag = "result", content = "value", rename_all = "snake_case")]
pub enum ChannelInteractionResult {
    Completed,
    Sent(Box<crate::channel::ChannelSendOutcome>),
    Draft(Option<String>),
    Info(ChannelInteractionInfo),
    TypingRefresh { handle: String },
}

#[cfg(test)]
#[path = "module_interaction_tests.rs"]
mod tests;
