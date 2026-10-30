//! Portable progressive output and managed transport vocabulary.
use crate::{ChannelRef, ConversationRef, OutboundContentPart};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use serde_json::Value;

#[derive(Clone, Serialize, Deserialize, JsonSchema)]
#[serde(tag = "event", content = "value", rename_all = "snake_case")]
pub enum ProgressiveEvent {
    TextDelta(String),
    ThinkingDelta(String),
    ToolCall(String),
    ToolResult { tool_name: String, success: bool },
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum ProgressiveOutputFailure {
    ProviderTransport,
    ProviderRejected,
    HostUnavailable,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct ToolFailureSummary {
    pub class: String,
    pub category: String,
    pub cause_plain: String,
    pub next_action: String,
    pub recoverable: bool,
}

#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(default, rename_all = "camelCase")]
pub struct SubagentUsage {
    pub input_tokens: u64,
    pub output_tokens: u64,
    pub cached_input_tokens: u64,
    pub charged_amount_usd: f64,
    pub cost_source: String,
}

#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(default, rename_all = "camelCase")]
pub struct SubagentOutput {
    pub agent_id: String,
    pub task_id: String,
    pub state: String,
    pub mode: Option<String>,
    pub dedicated_thread: Option<bool>,
    pub parent_call_id: Option<String>,
    pub call_id: Option<String>,
    pub iteration: Option<u32>,
    pub max_iterations: Option<u32>,
    pub prompt_chars: Option<usize>,
    pub message: Option<String>,
    pub display_name: Option<String>,
    pub display_label: Option<String>,
    pub display_detail: Option<String>,
    pub tool_name: Option<String>,
    pub success: Option<bool>,
    pub elapsed_ms: Option<u64>,
    pub output_chars: Option<usize>,
    pub output: Option<String>,
    pub arguments: Option<Value>,
    pub structured: Option<Value>,
    pub usage: Option<SubagentUsage>,
    pub failure: Option<ToolFailureSummary>,
    pub worker_thread_id: Option<String>,
    pub worktree_path: Option<String>,
    pub changed_files: Vec<String>,
    pub dirty_status: Option<bool>,
    pub stop: Option<String>,
    pub extended_policy: Option<bool>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(tag = "event", content = "value", rename_all = "snake_case")]
pub enum ChannelOutputEvent {
    TextDelta {
        text: String,
        iteration: Option<u32>,
    },
    ThinkingDelta {
        text: String,
        iteration: Option<u32>,
    },
    ToolCallArgsDelta {
        call_id: String,
        tool_name: String,
        delta: String,
        iteration: u32,
    },
    ToolCallProgress {
        call_id: String,
        message: String,
    },
    ToolCallStarted {
        call_id: String,
        tool_name: String,
        arguments: Value,
        iteration: u32,
        display_label: Option<String>,
        display_detail: Option<String>,
    },
    ToolCallCompleted {
        call_id: String,
        tool_name: String,
        success: bool,
        output_chars: usize,
        output: String,
        arguments: Option<Value>,
        elapsed_ms: u64,
        iteration: u32,
        failure: Option<ToolFailureSummary>,
        display_label: Option<String>,
        display_detail: Option<String>,
        structured: Option<Value>,
    },
    Lifecycle {
        stage: String,
        message: Option<String>,
        iteration: Option<u32>,
        max_iterations: Option<u32>,
        stop: Option<String>,
    },
    Usage {
        model: Option<String>,
        iteration: Option<u32>,
        input_tokens: u64,
        output_tokens: u64,
        cached_input_tokens: u64,
        total_usd: Option<f64>,
    },
    Subagent(Box<SubagentOutput>),
    FinalMessage {
        text: String,
        parts: Vec<crate::ContentPart>,
        receipt: Option<crate::channel::MessageReceipt>,
    },
    Media {
        text: Option<String>,
        parts: Vec<crate::ContentPart>,
    },
    VoiceNote {
        text: Option<String>,
        voice: crate::VoiceNote,
    },
    Clarification {
        prompt: String,
    },
}

#[derive(Clone, Serialize, Deserialize, JsonSchema)]
#[serde(tag = "operation", rename_all = "snake_case")]
pub enum ProgressiveCommand {
    Open {
        channel: String,
    },
    Feed {
        handle: String,
        events: Vec<ProgressiveEvent>,
    },
    Finish {
        handle: String,
        text: String,
    },
    Close {
        handle: String,
    },
    OpenOutput {
        channel: ChannelRef,
        conversation: ConversationRef,
        turn_id: String,
        lifetime_ms: u64,
    },
    OpenHostedOutput {
        channel: ChannelRef,
        conversation: ConversationRef,
        turn_id: String,
        lifetime_ms: u64,
    },
    FeedOutput {
        handle: String,
        turn_id: Option<String>,
        channel_ref: Option<ChannelRef>,
        conversation: Option<ConversationRef>,
        sequence: Option<u64>,
        events: Vec<ChannelOutputEvent>,
    },
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(tag = "result", content = "value", rename_all = "snake_case")]
pub enum ProgressiveResult {
    Opened {
        handle: String,
    },
    Updated,
    Closed,
    DeliveryOutcome {
        outcome: Box<crate::channel::ChannelSendOutcome>,
    },
    DeliveryOutcomes {
        outcomes: Vec<crate::channel::ChannelSendOutcome>,
    },
    DeliveryUnsupported {
        parts: Vec<OutboundContentPart>,
    },
    DeliveryBatch {
        outcomes: Vec<crate::channel::ChannelSendOutcome>,
        unsupported: Vec<OutboundContentPart>,
        failure: Option<ProgressiveOutputFailure>,
        hosted_accepted: bool,
    },
}

#[derive(Clone, Serialize, Deserialize, JsonSchema)]
#[serde(tag = "operation", rename_all = "snake_case")]
pub enum ProgressiveTransportRequest {
    Send {
        channel: String,
        turn_id: Option<String>,
        channel_ref: Option<ChannelRef>,
        conversation: Option<ConversationRef>,
        idempotency_key: Option<String>,
        message: Value,
    },
    SendTyped {
        channel: String,
        turn_id: Option<String>,
        channel_ref: Option<ChannelRef>,
        conversation: Option<ConversationRef>,
        sequence: Option<u64>,
        segment_index: Option<u32>,
        idempotency_key: Option<String>,
        parts: Vec<OutboundContentPart>,
        text: String,
    },
    Edit {
        channel: String,
        turn_id: Option<String>,
        channel_ref: Option<ChannelRef>,
        conversation: Option<ConversationRef>,
        message_id: String,
        receipt: Option<crate::channel::MessageReceipt>,
        text: String,
    },
    Delete {
        channel: String,
        channel_ref: Option<ChannelRef>,
        conversation: Option<ConversationRef>,
        receipt: Option<crate::channel::MessageReceipt>,
        message_id: String,
    },
    Typing {
        channel: String,
        channel_ref: Option<ChannelRef>,
        conversation: Option<ConversationRef>,
    },
    OutputEvent {
        channel: String,
        channel_ref: Option<ChannelRef>,
        conversation: Option<ConversationRef>,
        turn_id: String,
        sequence: u64,
        event: Box<ChannelOutputEvent>,
    },
}

#[derive(Clone, Serialize, Deserialize, JsonSchema)]
#[serde(tag = "result", content = "value", rename_all = "snake_case")]
pub enum ProgressiveTransportResult {
    Acknowledged,
    /// The hosted transport accepted responsibility for the event's delivery.
    Accepted,
    Outcome(Box<crate::channel::ChannelSendOutcome>),
    Success(Value),
    Unavailable,
    EditUnsupported,
    MessageGone,
    Failed,
}

#[cfg(test)]
#[path = "module_progressive_tests.rs"]
mod tests;
