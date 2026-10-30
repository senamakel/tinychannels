//! Shared web output payloads used by channel projections and host adapters.
use crate::ChannelEvent;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use serde_json::Value;

pub const MAX_WEB_TOOL_PAYLOAD_BYTES: usize = 64 * 1024;

/// Cap serialized tool arguments before they cross the web event bus.
pub fn cap_web_args(value: Option<Value>) -> Option<Value> {
    value.filter(|value| {
        serde_json::to_vec(value).map_or(false, |bytes| bytes.len() <= MAX_WEB_TOOL_PAYLOAD_BYTES)
    })
}

/// Cap tool output at a UTF-8 boundary while preserving a readable marker.
pub fn cap_web_output(value: String) -> String {
    if value.len() <= MAX_WEB_TOOL_PAYLOAD_BYTES {
        return value;
    }
    let marker = "… [output truncated]";
    let mut end = MAX_WEB_TOOL_PAYLOAD_BYTES.saturating_sub(marker.len());
    while !value.is_char_boundary(end) {
        end -= 1;
    }
    format!("{}{}", &value[..end], marker)
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(default, rename_all = "camelCase")]
pub struct WebEventContext {
    pub client_id: String,
    pub thread_id: String,
    pub request_id: String,
    pub turn_request_id: Option<String>,
    pub seq: Option<u64>,
    pub ts: Option<u64>,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum QueueMode {
    #[default]
    Interrupt,
    Steer,
    Followup,
    Collect,
    Parallel,
}

impl std::fmt::Display for QueueMode {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str(match self {
            Self::Interrupt => "interrupt",
            Self::Steer => "steer",
            Self::Followup => "followup",
            Self::Collect => "collect",
            Self::Parallel => "parallel",
        })
    }
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(default, rename_all = "camelCase")]
pub struct ChatRequestMetadata {
    pub source: Option<String>,
    pub queue_mode: Option<QueueMode>,
    pub task_id: Option<String>,
    pub speak_reply: Option<bool>,
    pub session_id: Option<u64>,
    pub run_mode: Option<String>,
    pub reasoning_effort: Option<String>,
    pub agent_id: Option<String>,
    pub hold_text_stream: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct ChatSuggestion {
    pub prompt: String,
    pub label: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct GuardrailReason {
    pub code: String,
    pub message: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct GuardrailPayload {
    pub verdict: String,
    pub score: f64,
    pub reasons: Vec<GuardrailReason>,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(default, rename_all = "camelCase")]
pub struct QueueItemPayload {
    pub id: String,
    pub lane: Option<String>,
    pub text_preview: Option<String>,
}

#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(default, rename_all = "camelCase")]
pub struct SubagentProgressDetail {
    pub task_id: String,
    pub agent_id: String,
    pub state: String,
    pub message: Option<String>,
    pub elapsed_ms: Option<u64>,
    pub output_chars: Option<usize>,
    pub usage: Option<SubagentUsagePayload>,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum WebCostSource {
    Charged,
    Estimated,
    #[default]
    Unknown,
}

#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(default, rename_all = "camelCase")]
pub struct TurnUsagePayload {
    pub input_tokens: u64,
    pub output_tokens: u64,
    pub cached_input_tokens: u64,
    pub cost_usd: Option<f64>,
    pub cost_source: WebCostSource,
    pub context_window: Option<u64>,
    pub context_tokens: Option<u64>,
    pub subagents: Vec<SubagentUsagePayload>,
}

#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(default, rename_all = "camelCase")]
pub struct SubagentUsagePayload {
    pub task_id: String,
    pub agent_id: String,
    pub input_tokens: u64,
    pub output_tokens: u64,
    pub cost_usd: Option<f64>,
}

#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(default, rename_all = "camelCase")]
pub struct TurnTimingPayload {
    pub elapsed_ms: u64,
    pub output_tokens: Option<u64>,
    pub tokens_per_second: Option<f64>,
    pub first_token_ms: Option<u64>,
    pub first_tool_ms: Option<u64>,
    pub round_one_narration_chars: usize,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(default, rename_all = "camelCase")]
pub struct TurnTimingSnapshot {
    pub total_ms: Option<u64>,
    pub first_token_ms: Option<u64>,
    pub first_tool_ms: Option<u64>,
    pub round_one_narration_chars: usize,
    pub tool_calls: u32,
}

impl TurnTimingSnapshot {
    /// Convert the captured turn timings to the stable terminal-event payload.
    pub fn into_payload(self, output_tokens: Option<u64>) -> TurnTimingPayload {
        let elapsed_ms = self.total_ms.unwrap_or_default();
        let tokens_per_second = output_tokens.and_then(|tokens| {
            (elapsed_ms > 0).then(|| tokens as f64 * 1_000.0 / elapsed_ms as f64)
        });
        TurnTimingPayload {
            elapsed_ms,
            output_tokens,
            tokens_per_second,
            first_token_ms: self.first_token_ms,
            first_tool_ms: self.first_tool_ms,
            round_one_narration_chars: self.round_one_narration_chars,
        }
    }
}

#[derive(Debug, Clone, Default)]
pub struct TurnTimingState {
    pub first_token_ms: Option<u64>,
    pub first_tool_ms: Option<u64>,
    pub round_one_narration_chars: usize,
    pub tool_calls: u32,
}

impl TurnTimingState {
    pub fn text_delta(&mut self, elapsed_ms: u64, text: &str, iteration: u32) -> bool {
        if self.first_token_ms.is_none() {
            self.first_token_ms = Some(elapsed_ms);
        }
        if iteration == 1 {
            self.round_one_narration_chars = self.round_one_narration_chars.saturating_add(text.chars().count());
        }
        true
    }
    pub fn tool_call(&mut self, elapsed_ms: u64) -> bool {
        self.first_tool_ms.get_or_insert(elapsed_ms);
        self.tool_calls = self.tool_calls.saturating_add(1);
        true
    }
    pub fn snapshot(&self, elapsed_ms: u64) -> TurnTimingSnapshot {
        TurnTimingSnapshot {
            total_ms: Some(elapsed_ms),
            first_token_ms: self.first_token_ms,
            first_tool_ms: self.first_tool_ms,
            round_one_narration_chars: self.round_one_narration_chars,
            tool_calls: self.tool_calls,
        }
    }
}

#[derive(Debug, Clone, Default)]
pub struct TurnCostThrottle {
    last_emit_ms: Option<u64>,
}
impl TurnCostThrottle {
    pub fn should_emit(&mut self, elapsed_ms: u64) -> bool {
        let emit = self
            .last_emit_ms
            .is_none_or(|last| elapsed_ms.saturating_sub(last) >= 1_000);
        if emit {
            self.last_emit_ms = Some(elapsed_ms);
        }
        emit
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(tag = "event", rename_all = "snake_case")]
pub enum WebChannelEvent {
    TextDelta {
        context: WebEventContext,
        delta: Option<String>,
        delta_kind: Option<String>,
        round: Option<u32>,
    },
    ChatDone {
        context: WebEventContext,
        full_response: Option<String>,
        usage: Option<TurnUsagePayload>,
        timing: Option<TurnTimingPayload>,
        citations: Option<Vec<Value>>,
        parts: Option<Vec<crate::ContentPart>>,
    },
    ChatError {
        context: WebEventContext,
        message: Option<String>,
        error_type: Option<String>,
        error_source: Option<String>,
        error_retryable: Option<bool>,
        error_retry_after_ms: Option<u64>,
        error_provider: Option<String>,
        error_fallback_available: Option<bool>,
        copy_key: Option<String>,
        copy_params: Option<Value>,
        guardrail: Option<GuardrailPayload>,
    },
    ChatCancelled {
        context: WebEventContext,
        cancel_reason: Option<String>,
        superseded_by: Option<String>,
    },
    ChatInterim { context: WebEventContext, full_response: Option<String>, round: Option<u32> },
    ChatSuggestions { context: WebEventContext, suggestions: Vec<ChatSuggestion> },
    UserError { context: WebEventContext, message: Option<String>, error_type: Option<String>, error_source: Option<String>, error_retryable: Option<bool> },
    ToolCall { context: WebEventContext, tool_name: Option<String>, tool_call_id: Option<String>, args: Option<Value> },
    ToolResult { context: WebEventContext, tool_name: Option<String>, tool_call_id: Option<String>, success: Option<bool>, output: Option<String>, elapsed_ms: Option<u64>, output_chars: Option<usize>, structured: Option<Value> },
    ToolProgress { context: WebEventContext, tool_call_id: Option<String>, message: Option<String> },
    ThinkingDelta { context: WebEventContext, delta: Option<String> },
    ProactiveMessage { context: WebEventContext, full_response: Option<String>, message: Option<String>, success: Option<bool>, persisted_message_id: Option<String>, parts: Option<Vec<crate::ContentPart>> },
    QueueItemRemoved { context: WebEventContext, queue_item: Option<QueueItemPayload> },
    QueueItemQueued { context: WebEventContext, queue_item: Option<QueueItemPayload> },
    QueueItemDelivered { context: WebEventContext, queue_item: Option<QueueItemPayload> },
    ChannelOutbound { context: WebEventContext, full_response: Option<String>, structured: Option<Value> },
    ExternalTransferPending { context: WebEventContext, args: Option<Value> },
    MemoryActivity { context: WebEventContext, args: Option<Value> },
    ThreadGoalUpdated { context: WebEventContext, goal: Option<Value> },
    ThreadGoalCleared { context: WebEventContext },
    ThreadTodosChanged { context: WebEventContext, todos: Option<Value> },
    RunModeChanged { context: WebEventContext, message: Option<String> },
    ArtifactReady { context: WebEventContext, tool_call_id: Option<String>, args: Option<Value> },
    ArtifactFailed { context: WebEventContext, tool_call_id: Option<String>, args: Option<Value> },
    ArtifactPending { context: WebEventContext, tool_call_id: Option<String>, args: Option<Value> },
    ApprovalRequest { context: WebEventContext, tool_name: Option<String>, message: Option<String>, args: Option<Value>, tool_call_id: Option<String>, expires_at: Option<String>, detached: Option<bool> },
    PlanReviewRequest { context: WebEventContext, tool_name: Option<String>, message: Option<String>, args: Option<Value>, tool_call_id: Option<String>, expires_at: Option<String> },
    ApprovalDecided { context: WebEventContext, tool_name: Option<String>, message: Option<String>, tool_call_id: Option<String>, cancel_reason: Option<String> },
    PlanReviewDecided { context: WebEventContext, tool_name: Option<String>, message: Option<String>, tool_call_id: Option<String>, cancel_reason: Option<String> },
    VoiceSpeak { context: WebEventContext, full_response: Option<String>, success: Option<bool> },
    ChannelEvent {
        context: WebEventContext,
        openhuman_message_id: Option<String>,
        channel_event: ChannelEvent,
    },
}

impl WebChannelEvent {
    pub fn context(&self) -> &WebEventContext {
        match self {
            Self::TextDelta { context, .. }
            | Self::ChatDone { context, .. }
            | Self::ChatError { context, .. }
            | Self::ChatCancelled { context, .. }
            | Self::ChannelEvent { context, .. }
            | Self::ChatInterim { context, .. }
            | Self::ChatSuggestions { context, .. }
            | Self::UserError { context, .. }
            | Self::ToolCall { context, .. }
            | Self::ToolResult { context, .. }
            | Self::ThinkingDelta { context, .. }
            | Self::ProactiveMessage { context, .. }
            | Self::QueueItemRemoved { context, .. }
            | Self::ChannelOutbound { context, .. }
            | Self::ToolProgress { context, .. }
            | Self::QueueItemQueued { context, .. }
            | Self::QueueItemDelivered { context, .. }
            | Self::ExternalTransferPending { context, .. }
            | Self::MemoryActivity { context, .. }
            | Self::ThreadGoalUpdated { context, .. }
            | Self::ThreadGoalCleared { context, .. }
            | Self::ThreadTodosChanged { context, .. }
            | Self::RunModeChanged { context, .. }
            | Self::ArtifactReady { context, .. }
            | Self::ArtifactFailed { context, .. }
            | Self::ArtifactPending { context, .. }
            | Self::ApprovalRequest { context, .. }
            | Self::PlanReviewRequest { context, .. }
            | Self::ApprovalDecided { context, .. }
            | Self::PlanReviewDecided { context, .. }
            | Self::VoiceSpeak { context, .. } => context,
        }
    }
    pub fn context_mut(&mut self) -> &mut WebEventContext {
        match self {
            Self::TextDelta { context, .. } | Self::ChatDone { context, .. }
            | Self::ChatError { context, .. } | Self::ChatCancelled { context, .. }
            | Self::ChannelEvent { context, .. } | Self::ChatInterim { context, .. }
            | Self::ChatSuggestions { context, .. } | Self::UserError { context, .. }
            | Self::ToolCall { context, .. } | Self::ToolResult { context, .. }
            | Self::ThinkingDelta { context, .. } | Self::ProactiveMessage { context, .. }
            | Self::QueueItemRemoved { context, .. } | Self::ChannelOutbound { context, .. } => context,
            Self::ToolProgress { context, .. } | Self::QueueItemQueued { context, .. }
            | Self::QueueItemDelivered { context, .. } | Self::ExternalTransferPending { context, .. }
            | Self::MemoryActivity { context, .. } | Self::ThreadGoalUpdated { context, .. }
            | Self::ThreadGoalCleared { context, .. } | Self::ThreadTodosChanged { context, .. }
            | Self::RunModeChanged { context, .. } | Self::ArtifactReady { context, .. }
            | Self::ArtifactFailed { context, .. } | Self::ArtifactPending { context, .. }
            | Self::ApprovalRequest { context, .. } | Self::PlanReviewRequest { context, .. }
            | Self::ApprovalDecided { context, .. } | Self::PlanReviewDecided { context, .. }
            | Self::VoiceSpeak { context, .. } => context,
        }
    }
    pub fn normalize_projected_wire_fields(&mut self) {}
    pub fn event_name(&self) -> &'static str {
        match self {
            Self::TextDelta { .. } => "text_delta",
            Self::ChatDone { .. } => "chat_done",
            Self::ChatError { .. } => "chat_error",
            Self::ChatCancelled { .. } => "chat_cancelled",
            Self::ChannelEvent { .. } => "channel_event",
            Self::ChatInterim { .. } => "chat_interim",
            Self::ChatSuggestions { .. } => "chat_suggestions",
            Self::UserError { .. } => "user_error",
            Self::ToolCall { .. } => "tool_call",
            Self::ToolResult { .. } => "tool_result",
            Self::ThinkingDelta { .. } => "thinking_delta",
            Self::ProactiveMessage { .. } => "proactive_message",
            Self::QueueItemRemoved { .. } => "queue_item_removed",
            Self::ChannelOutbound { .. } => "channel_outbound",
            Self::ToolProgress { .. } => "tool_progress",
            Self::QueueItemQueued { .. } => "queue_item_queued",
            Self::QueueItemDelivered { .. } => "queue_item_delivered",
            Self::ExternalTransferPending { .. } => "external_transfer_pending",
            Self::MemoryActivity { .. } => "memory_activity",
            Self::ThreadGoalUpdated { .. } => "thread_goal_updated",
            Self::ThreadGoalCleared { .. } => "thread_goal_cleared",
            Self::ThreadTodosChanged { .. } => "thread_todos_changed",
            Self::RunModeChanged { .. } => "run_mode_changed",
            Self::ArtifactReady { .. } => "artifact_ready",
            Self::ArtifactFailed { .. } => "artifact_failed",
            Self::ArtifactPending { .. } => "artifact_pending",
            Self::ApprovalRequest { .. } => "approval_request",
            Self::PlanReviewRequest { .. } => "plan_review_request",
            Self::ApprovalDecided { .. } => "approval_decided",
            Self::PlanReviewDecided { .. } => "plan_review_decided",
            Self::VoiceSpeak { .. } => "voice_speak",
        }
    }
}

#[cfg(test)]
#[path = "web_tests.rs"]
mod tests;
