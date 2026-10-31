//! Shared web output payloads used by channel projections and host adapters.
use crate::ChannelEvent;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use serde_json::Value;

pub const MAX_WEB_TOOL_PAYLOAD_BYTES: usize = 64 * 1024;

/// Cap serialized tool arguments before they cross the web event bus.
pub fn cap_web_args(value: Option<Value>) -> Option<Value> {
    value.filter(|value| {
        serde_json::to_vec(value).is_ok_and(|bytes| bytes.len() <= MAX_WEB_TOOL_PAYLOAD_BYTES)
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
#[serde(default, rename_all = "snake_case")]
pub struct WebEventContext {
    pub client_id: String,
    pub thread_id: String,
    pub request_id: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub turn_request_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub seq: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
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
#[serde(default, rename_all = "snake_case")]
pub struct SubagentProgressDetail {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub mode: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub dedicated_thread: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub prompt_chars: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub child_iteration: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub child_max_iterations: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub agent_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub task_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub elapsed_ms: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub input_tokens: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub output_tokens: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cached_input_tokens: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cost_usd: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub iterations: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub output_chars: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub worker_thread_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub display_name: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub worktree_path: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub changed_files: Option<Vec<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub dirty_status: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub parent_call_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub output: Option<String>,
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
#[serde(default, rename_all = "snake_case")]
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
#[serde(default, rename_all = "snake_case")]
pub struct SubagentUsagePayload {
    pub task_id: String,
    pub agent_id: String,
    pub input_tokens: u64,
    pub output_tokens: u64,
    pub cost_usd: Option<f64>,
}

#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(default, rename_all = "snake_case")]
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
            self.round_one_narration_chars = self
                .round_one_narration_chars
                .saturating_add(text.chars().count());
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
/// Addressed output events omit absent optional fields while retaining present
/// false, zero, and explicit JSON payload values.
pub enum WebChannelEvent {
    TextDelta {
        #[serde(flatten)]
        context: WebEventContext,
        #[serde(skip_serializing_if = "Option::is_none")]
        delta: Option<String>,
        #[serde(skip_serializing_if = "Option::is_none")]
        delta_kind: Option<String>,
        #[serde(skip_serializing_if = "Option::is_none")]
        round: Option<u32>,
    },
    ChatDone {
        #[serde(flatten)]
        context: WebEventContext,
        #[serde(skip_serializing_if = "Option::is_none")]
        full_response: Option<String>,
        #[serde(skip_serializing_if = "Option::is_none")]
        usage: Option<TurnUsagePayload>,
        #[serde(skip_serializing_if = "Option::is_none")]
        timing: Option<TurnTimingPayload>,
        #[serde(skip_serializing_if = "Option::is_none")]
        citations: Option<Vec<Value>>,
        #[serde(skip_serializing_if = "Option::is_none")]
        parts: Option<Vec<crate::ContentPart>>,
    },
    ChatError {
        #[serde(flatten)]
        context: WebEventContext,
        #[serde(skip_serializing_if = "Option::is_none")]
        message: Option<String>,
        #[serde(skip_serializing_if = "Option::is_none")]
        error_type: Option<String>,
        #[serde(skip_serializing_if = "Option::is_none")]
        error_source: Option<String>,
        #[serde(skip_serializing_if = "Option::is_none")]
        error_retryable: Option<bool>,
        #[serde(skip_serializing_if = "Option::is_none")]
        error_retry_after_ms: Option<u64>,
        #[serde(skip_serializing_if = "Option::is_none")]
        error_provider: Option<String>,
        #[serde(skip_serializing_if = "Option::is_none")]
        error_fallback_available: Option<bool>,
        #[serde(skip_serializing_if = "Option::is_none")]
        copy_key: Option<String>,
        #[serde(skip_serializing_if = "Option::is_none")]
        copy_params: Option<Value>,
        #[serde(skip_serializing_if = "Option::is_none")]
        guardrail: Option<GuardrailPayload>,
    },
    ChatCancelled {
        #[serde(flatten)]
        context: WebEventContext,
        #[serde(skip_serializing_if = "Option::is_none")]
        cancel_reason: Option<String>,
        #[serde(skip_serializing_if = "Option::is_none")]
        superseded_by: Option<String>,
    },
    ChatInterim {
        #[serde(flatten)]
        context: WebEventContext,
        #[serde(skip_serializing_if = "Option::is_none")]
        full_response: Option<String>,
        #[serde(skip_serializing_if = "Option::is_none")]
        round: Option<u32>,
    },
    ChatSuggestions {
        #[serde(flatten)]
        context: WebEventContext,
        suggestions: Vec<ChatSuggestion>,
    },
    UserError {
        #[serde(flatten)]
        context: WebEventContext,
        #[serde(skip_serializing_if = "Option::is_none")]
        message: Option<String>,
        #[serde(skip_serializing_if = "Option::is_none")]
        error_type: Option<String>,
        #[serde(skip_serializing_if = "Option::is_none")]
        error_source: Option<String>,
        #[serde(skip_serializing_if = "Option::is_none")]
        error_retryable: Option<bool>,
    },
    ToolCall {
        #[serde(flatten)]
        context: WebEventContext,
        #[serde(skip_serializing_if = "Option::is_none")]
        tool_name: Option<String>,
        #[serde(skip_serializing_if = "Option::is_none")]
        tool_call_id: Option<String>,
        #[serde(skip_serializing_if = "Option::is_none")]
        args: Option<Value>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        skill_id: Option<String>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        round: Option<u32>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        tool_display_label: Option<String>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        tool_display_detail: Option<String>,
    },
    ToolResult {
        #[serde(flatten)]
        context: WebEventContext,
        #[serde(skip_serializing_if = "Option::is_none")]
        tool_name: Option<String>,
        #[serde(skip_serializing_if = "Option::is_none")]
        tool_call_id: Option<String>,
        #[serde(skip_serializing_if = "Option::is_none")]
        success: Option<bool>,
        #[serde(skip_serializing_if = "Option::is_none")]
        output: Option<String>,
        #[serde(skip_serializing_if = "Option::is_none")]
        elapsed_ms: Option<u64>,
        #[serde(skip_serializing_if = "Option::is_none")]
        output_chars: Option<usize>,
        #[serde(skip_serializing_if = "Option::is_none")]
        structured: Option<Value>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        skill_id: Option<String>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        args: Option<serde_json::Value>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        round: Option<u32>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        failure: Option<serde_json::Value>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        tool_display_label: Option<String>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        tool_display_detail: Option<String>,
    },
    ToolProgress {
        #[serde(flatten)]
        context: WebEventContext,
        #[serde(skip_serializing_if = "Option::is_none")]
        tool_call_id: Option<String>,
        #[serde(skip_serializing_if = "Option::is_none")]
        message: Option<String>,
    },
    ThinkingDelta {
        #[serde(flatten)]
        context: WebEventContext,
        #[serde(skip_serializing_if = "Option::is_none")]
        delta: Option<String>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        delta_kind: Option<String>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        round: Option<u32>,
    },
    ProactiveMessage {
        #[serde(flatten)]
        context: WebEventContext,
        #[serde(skip_serializing_if = "Option::is_none")]
        full_response: Option<String>,
        #[serde(skip_serializing_if = "Option::is_none")]
        message: Option<String>,
        #[serde(skip_serializing_if = "Option::is_none")]
        success: Option<bool>,
        #[serde(skip_serializing_if = "Option::is_none")]
        persisted_message_id: Option<String>,
        #[serde(skip_serializing_if = "Option::is_none")]
        parts: Option<Vec<crate::ContentPart>>,
    },
    QueueItemRemoved {
        #[serde(flatten)]
        context: WebEventContext,
        #[serde(skip_serializing_if = "Option::is_none")]
        queue_item: Option<QueueItemPayload>,
    },
    QueueItemQueued {
        #[serde(flatten)]
        context: WebEventContext,
        #[serde(skip_serializing_if = "Option::is_none")]
        queue_item: Option<QueueItemPayload>,
    },
    QueueItemDelivered {
        #[serde(flatten)]
        context: WebEventContext,
        #[serde(skip_serializing_if = "Option::is_none")]
        queue_item: Option<QueueItemPayload>,
    },
    ChannelOutbound {
        #[serde(flatten)]
        context: WebEventContext,
        #[serde(skip_serializing_if = "Option::is_none")]
        full_response: Option<String>,
        #[serde(skip_serializing_if = "Option::is_none")]
        structured: Option<Value>,
    },
    ExternalTransferPending {
        #[serde(flatten)]
        context: WebEventContext,
        #[serde(skip_serializing_if = "Option::is_none")]
        args: Option<Value>,
    },
    MemoryActivity {
        #[serde(flatten)]
        context: WebEventContext,
        #[serde(skip_serializing_if = "Option::is_none")]
        args: Option<Value>,
    },
    ThreadGoalUpdated {
        #[serde(flatten)]
        context: WebEventContext,
        #[serde(skip_serializing_if = "Option::is_none")]
        goal: Option<Value>,
    },
    ThreadGoalCleared {
        #[serde(flatten)]
        context: WebEventContext,
    },
    ThreadTodosChanged {
        #[serde(flatten)]
        context: WebEventContext,
        #[serde(skip_serializing_if = "Option::is_none")]
        todos: Option<Value>,
    },
    RunModeChanged {
        #[serde(flatten)]
        context: WebEventContext,
        #[serde(skip_serializing_if = "Option::is_none")]
        message: Option<String>,
    },
    ArtifactReady {
        #[serde(flatten)]
        context: WebEventContext,
        #[serde(skip_serializing_if = "Option::is_none")]
        tool_call_id: Option<String>,
        #[serde(skip_serializing_if = "Option::is_none")]
        args: Option<Value>,
    },
    ArtifactFailed {
        #[serde(flatten)]
        context: WebEventContext,
        #[serde(skip_serializing_if = "Option::is_none")]
        tool_call_id: Option<String>,
        #[serde(skip_serializing_if = "Option::is_none")]
        args: Option<Value>,
    },
    ArtifactPending {
        #[serde(flatten)]
        context: WebEventContext,
        #[serde(skip_serializing_if = "Option::is_none")]
        tool_call_id: Option<String>,
        #[serde(skip_serializing_if = "Option::is_none")]
        args: Option<Value>,
    },
    ApprovalRequest {
        #[serde(flatten)]
        context: WebEventContext,
        #[serde(skip_serializing_if = "Option::is_none")]
        tool_name: Option<String>,
        #[serde(skip_serializing_if = "Option::is_none")]
        message: Option<String>,
        #[serde(skip_serializing_if = "Option::is_none")]
        args: Option<Value>,
        #[serde(skip_serializing_if = "Option::is_none")]
        tool_call_id: Option<String>,
        #[serde(skip_serializing_if = "Option::is_none")]
        expires_at: Option<String>,
        #[serde(skip_serializing_if = "Option::is_none")]
        detached: Option<bool>,
    },
    PlanReviewRequest {
        #[serde(flatten)]
        context: WebEventContext,
        #[serde(skip_serializing_if = "Option::is_none")]
        tool_name: Option<String>,
        #[serde(skip_serializing_if = "Option::is_none")]
        message: Option<String>,
        #[serde(skip_serializing_if = "Option::is_none")]
        args: Option<Value>,
        #[serde(skip_serializing_if = "Option::is_none")]
        tool_call_id: Option<String>,
        #[serde(skip_serializing_if = "Option::is_none")]
        expires_at: Option<String>,
    },
    ApprovalDecided {
        #[serde(flatten)]
        context: WebEventContext,
        #[serde(skip_serializing_if = "Option::is_none")]
        tool_name: Option<String>,
        #[serde(skip_serializing_if = "Option::is_none")]
        message: Option<String>,
        #[serde(skip_serializing_if = "Option::is_none")]
        tool_call_id: Option<String>,
        #[serde(skip_serializing_if = "Option::is_none")]
        cancel_reason: Option<String>,
    },
    PlanReviewDecided {
        #[serde(flatten)]
        context: WebEventContext,
        #[serde(skip_serializing_if = "Option::is_none")]
        tool_name: Option<String>,
        #[serde(skip_serializing_if = "Option::is_none")]
        message: Option<String>,
        #[serde(skip_serializing_if = "Option::is_none")]
        tool_call_id: Option<String>,
        #[serde(skip_serializing_if = "Option::is_none")]
        cancel_reason: Option<String>,
    },
    VoiceSpeak {
        #[serde(flatten)]
        context: WebEventContext,
        #[serde(skip_serializing_if = "Option::is_none")]
        full_response: Option<String>,
        #[serde(skip_serializing_if = "Option::is_none")]
        success: Option<bool>,
    },
    ChannelEvent {
        #[serde(flatten)]
        context: WebEventContext,
        #[serde(skip_serializing_if = "Option::is_none")]
        openhuman_message_id: Option<String>,
        channel_event: ChannelEvent,
    },
    ToolArgsDelta {
        #[serde(flatten)]
        context: WebEventContext,
        #[serde(skip_serializing_if = "Option::is_none")]
        delta: Option<String>,
        #[serde(skip_serializing_if = "Option::is_none")]
        delta_kind: Option<String>,
        #[serde(skip_serializing_if = "Option::is_none")]
        tool_call_id: Option<String>,
        #[serde(skip_serializing_if = "Option::is_none")]
        tool_name: Option<String>,
        #[serde(skip_serializing_if = "Option::is_none")]
        skill_id: Option<String>,
        #[serde(skip_serializing_if = "Option::is_none")]
        round: Option<u32>,
    },
    InferenceStart {
        #[serde(flatten)]
        context: WebEventContext,
    },
    InferenceHeartbeat {
        #[serde(flatten)]
        context: WebEventContext,
    },
    IterationStart {
        #[serde(flatten)]
        context: WebEventContext,
        #[serde(skip_serializing_if = "Option::is_none")]
        message: Option<String>,
        #[serde(skip_serializing_if = "Option::is_none")]
        round: Option<u32>,
    },
    ContextCompacting {
        #[serde(flatten)]
        context: WebEventContext,
        #[serde(skip_serializing_if = "Option::is_none")]
        round: Option<u32>,
    },
    TurnCost {
        #[serde(flatten)]
        context: WebEventContext,
        #[serde(skip_serializing_if = "Option::is_none")]
        round: Option<u32>,
        #[serde(skip_serializing_if = "Option::is_none")]
        usage: Option<TurnUsagePayload>,
    },
    SubagentSpawned {
        #[serde(flatten)]
        context: WebEventContext,
        #[serde(skip_serializing_if = "Option::is_none")]
        message: Option<String>,
        #[serde(skip_serializing_if = "Option::is_none")]
        tool_name: Option<String>,
        #[serde(skip_serializing_if = "Option::is_none")]
        skill_id: Option<String>,
        #[serde(skip_serializing_if = "Option::is_none")]
        round: Option<u32>,
        #[serde(skip_serializing_if = "Option::is_none")]
        subagent: Option<SubagentProgressDetail>,
    },
    SubagentCompleted {
        #[serde(flatten)]
        context: WebEventContext,
        #[serde(skip_serializing_if = "Option::is_none")]
        message: Option<String>,
        #[serde(skip_serializing_if = "Option::is_none")]
        tool_name: Option<String>,
        #[serde(skip_serializing_if = "Option::is_none")]
        skill_id: Option<String>,
        #[serde(skip_serializing_if = "Option::is_none")]
        success: Option<bool>,
        #[serde(skip_serializing_if = "Option::is_none")]
        round: Option<u32>,
        #[serde(skip_serializing_if = "Option::is_none")]
        subagent: Option<SubagentProgressDetail>,
    },
    SubagentFailed {
        #[serde(flatten)]
        context: WebEventContext,
        #[serde(skip_serializing_if = "Option::is_none")]
        message: Option<String>,
        #[serde(skip_serializing_if = "Option::is_none")]
        tool_name: Option<String>,
        #[serde(skip_serializing_if = "Option::is_none")]
        skill_id: Option<String>,
        #[serde(skip_serializing_if = "Option::is_none")]
        success: Option<bool>,
        #[serde(skip_serializing_if = "Option::is_none")]
        subagent: Option<SubagentProgressDetail>,
        #[serde(skip_serializing_if = "Option::is_none")]
        round: Option<u32>,
    },
    SubagentAwaitingUser {
        #[serde(flatten)]
        context: WebEventContext,
        #[serde(skip_serializing_if = "Option::is_none")]
        message: Option<String>,
        #[serde(skip_serializing_if = "Option::is_none")]
        tool_name: Option<String>,
        #[serde(skip_serializing_if = "Option::is_none")]
        skill_id: Option<String>,
        #[serde(skip_serializing_if = "Option::is_none")]
        success: Option<bool>,
        #[serde(skip_serializing_if = "Option::is_none")]
        round: Option<u32>,
        #[serde(skip_serializing_if = "Option::is_none")]
        subagent: Option<SubagentProgressDetail>,
    },
    SubagentIterationStart {
        #[serde(flatten)]
        context: WebEventContext,
        #[serde(skip_serializing_if = "Option::is_none")]
        message: Option<String>,
        #[serde(skip_serializing_if = "Option::is_none")]
        tool_name: Option<String>,
        #[serde(skip_serializing_if = "Option::is_none")]
        skill_id: Option<String>,
        #[serde(skip_serializing_if = "Option::is_none")]
        round: Option<u32>,
        #[serde(skip_serializing_if = "Option::is_none")]
        subagent: Option<SubagentProgressDetail>,
    },
    SubagentToolCall {
        #[serde(flatten)]
        context: WebEventContext,
        #[serde(skip_serializing_if = "Option::is_none")]
        tool_name: Option<String>,
        #[serde(skip_serializing_if = "Option::is_none")]
        skill_id: Option<String>,
        #[serde(skip_serializing_if = "Option::is_none")]
        args: Option<serde_json::Value>,
        #[serde(skip_serializing_if = "Option::is_none")]
        round: Option<u32>,
        #[serde(skip_serializing_if = "Option::is_none")]
        tool_call_id: Option<String>,
        #[serde(skip_serializing_if = "Option::is_none")]
        tool_display_label: Option<String>,
        #[serde(skip_serializing_if = "Option::is_none")]
        tool_display_detail: Option<String>,
        #[serde(skip_serializing_if = "Option::is_none")]
        subagent: Option<SubagentProgressDetail>,
    },
    SubagentToolResult {
        #[serde(flatten)]
        context: WebEventContext,
        #[serde(skip_serializing_if = "Option::is_none")]
        tool_name: Option<String>,
        #[serde(skip_serializing_if = "Option::is_none")]
        skill_id: Option<String>,
        #[serde(skip_serializing_if = "Option::is_none")]
        success: Option<bool>,
        #[serde(skip_serializing_if = "Option::is_none")]
        round: Option<u32>,
        #[serde(skip_serializing_if = "Option::is_none")]
        tool_call_id: Option<String>,
        #[serde(skip_serializing_if = "Option::is_none")]
        output: Option<String>,
        #[serde(skip_serializing_if = "Option::is_none")]
        args: Option<serde_json::Value>,
        #[serde(skip_serializing_if = "Option::is_none")]
        elapsed_ms: Option<u64>,
        #[serde(skip_serializing_if = "Option::is_none")]
        tool_display_label: Option<String>,
        #[serde(skip_serializing_if = "Option::is_none")]
        tool_display_detail: Option<String>,
        #[serde(skip_serializing_if = "Option::is_none")]
        structured: Option<serde_json::Value>,
        #[serde(skip_serializing_if = "Option::is_none")]
        failure: Option<serde_json::Value>,
        #[serde(skip_serializing_if = "Option::is_none")]
        subagent: Option<SubagentProgressDetail>,
    },
    SubagentTextDelta {
        #[serde(flatten)]
        context: WebEventContext,
        #[serde(skip_serializing_if = "Option::is_none")]
        delta: Option<String>,
        #[serde(skip_serializing_if = "Option::is_none")]
        delta_kind: Option<String>,
        #[serde(skip_serializing_if = "Option::is_none")]
        round: Option<u32>,
        #[serde(skip_serializing_if = "Option::is_none")]
        skill_id: Option<String>,
        #[serde(skip_serializing_if = "Option::is_none")]
        subagent: Option<SubagentProgressDetail>,
    },
    SubagentThinkingDelta {
        #[serde(flatten)]
        context: WebEventContext,
        #[serde(skip_serializing_if = "Option::is_none")]
        delta: Option<String>,
        #[serde(skip_serializing_if = "Option::is_none")]
        delta_kind: Option<String>,
        #[serde(skip_serializing_if = "Option::is_none")]
        round: Option<u32>,
        #[serde(skip_serializing_if = "Option::is_none")]
        skill_id: Option<String>,
        #[serde(skip_serializing_if = "Option::is_none")]
        subagent: Option<SubagentProgressDetail>,
    },
    ChatMessage {
        #[serde(flatten)]
        context: WebEventContext,
        #[serde(skip_serializing_if = "Option::is_none")]
        message: Option<String>,
        #[serde(skip_serializing_if = "Option::is_none")]
        full_response: Option<String>,
        #[serde(skip_serializing_if = "Option::is_none")]
        parts: Option<Vec<crate::ContentPart>>,
    },
    ChatSegment {
        #[serde(flatten)]
        context: WebEventContext,
        #[serde(skip_serializing_if = "Option::is_none")]
        message: Option<String>,
        #[serde(skip_serializing_if = "Option::is_none")]
        full_response: Option<String>,
        #[serde(skip_serializing_if = "Option::is_none")]
        segment_index: Option<u32>,
        #[serde(skip_serializing_if = "Option::is_none")]
        segment_total: Option<u32>,
        #[serde(skip_serializing_if = "Option::is_none")]
        parts: Option<Vec<crate::ContentPart>>,
    },
}

impl WebChannelEvent {
    pub fn context(&self) -> &WebEventContext {
        match self {
            Self::ToolArgsDelta { context, .. }
            | Self::InferenceStart { context, .. }
            | Self::InferenceHeartbeat { context, .. }
            | Self::IterationStart { context, .. }
            | Self::ContextCompacting { context, .. }
            | Self::TurnCost { context, .. }
            | Self::SubagentSpawned { context, .. }
            | Self::SubagentCompleted { context, .. }
            | Self::SubagentFailed { context, .. }
            | Self::SubagentAwaitingUser { context, .. }
            | Self::SubagentIterationStart { context, .. }
            | Self::SubagentToolCall { context, .. }
            | Self::SubagentToolResult { context, .. }
            | Self::SubagentTextDelta { context, .. }
            | Self::SubagentThinkingDelta { context, .. }
            | Self::ChatMessage { context, .. }
            | Self::ChatSegment { context, .. }
            | Self::TextDelta { context, .. }
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
            Self::ToolArgsDelta { context, .. }
            | Self::InferenceStart { context, .. }
            | Self::InferenceHeartbeat { context, .. }
            | Self::IterationStart { context, .. }
            | Self::ContextCompacting { context, .. }
            | Self::TurnCost { context, .. }
            | Self::SubagentSpawned { context, .. }
            | Self::SubagentCompleted { context, .. }
            | Self::SubagentFailed { context, .. }
            | Self::SubagentAwaitingUser { context, .. }
            | Self::SubagentIterationStart { context, .. }
            | Self::SubagentToolCall { context, .. }
            | Self::SubagentToolResult { context, .. }
            | Self::SubagentTextDelta { context, .. }
            | Self::SubagentThinkingDelta { context, .. }
            | Self::ChatMessage { context, .. }
            | Self::ChatSegment { context, .. }
            | Self::TextDelta { context, .. }
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
            | Self::ChannelOutbound { context, .. } => context,
            Self::ToolProgress { context, .. }
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
    pub fn normalize_projected_wire_fields(&mut self) {
        if let Self::ToolCall {
            tool_name, args, ..
        } = self
        {
            if tool_name.as_deref().is_some_and(str::is_empty) {
                *tool_name = None;
            }
            if args.as_ref().is_some_and(Value::is_null) {
                *args = None;
            }
        }
    }
    pub fn event_name(&self) -> &'static str {
        match self {
            Self::ToolArgsDelta { .. } => "tool_args_delta",
            Self::InferenceStart { .. } => "inference_start",
            Self::InferenceHeartbeat { .. } => "inference_heartbeat",
            Self::IterationStart { .. } => "iteration_start",
            Self::ContextCompacting { .. } => "context_compacting",
            Self::TurnCost { .. } => "turn_cost",
            Self::SubagentSpawned { .. } => "subagent_spawned",
            Self::SubagentCompleted { .. } => "subagent_completed",
            Self::SubagentFailed { .. } => "subagent_failed",
            Self::SubagentAwaitingUser { .. } => "subagent_awaiting_user",
            Self::SubagentIterationStart { .. } => "subagent_iteration_start",
            Self::SubagentToolCall { .. } => "subagent_tool_call",
            Self::SubagentToolResult { .. } => "subagent_tool_result",
            Self::SubagentTextDelta { .. } => "subagent_text_delta",
            Self::SubagentThinkingDelta { .. } => "subagent_thinking_delta",
            Self::ChatMessage { .. } => "chat_message",
            Self::ChatSegment { .. } => "chat_segment",
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
