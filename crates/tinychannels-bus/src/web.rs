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

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(default, rename_all = "camelCase")]
pub struct ChatRequestMetadata {
    pub source: Option<String>,
    pub queue_mode: Option<QueueMode>,
    pub task_id: Option<String>,
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
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(default, rename_all = "camelCase")]
pub struct TurnTimingSnapshot {
    pub total_ms: Option<u64>,
    pub first_token_ms: Option<u64>,
    pub tool_calls: u32,
}

#[derive(Debug, Clone, Default)]
pub struct TurnTimingState {
    pub first_token_ms: Option<u64>,
    pub tool_calls: u32,
}

impl TurnTimingState {
    pub fn text_delta(&mut self, elapsed_ms: u64, _text: &str, _iteration: u32) -> bool {
        if self.first_token_ms.is_none() {
            self.first_token_ms = Some(elapsed_ms);
        }
        true
    }
    pub fn tool_call(&mut self, _elapsed_ms: u64) -> bool {
        self.tool_calls = self.tool_calls.saturating_add(1);
        true
    }
    pub fn snapshot(&self, elapsed_ms: u64) -> TurnTimingSnapshot {
        TurnTimingSnapshot {
            total_ms: Some(elapsed_ms),
            first_token_ms: self.first_token_ms,
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
    },
    ChatError {
        context: WebEventContext,
        message: Option<String>,
        error_type: Option<String>,
    },
    ChatCancelled {
        context: WebEventContext,
        cancel_reason: Option<String>,
    },
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
            | Self::ChannelEvent { context, .. } => context,
        }
    }
    pub fn event_name(&self) -> &'static str {
        match self {
            Self::TextDelta { .. } => "text_delta",
            Self::ChatDone { .. } => "chat_done",
            Self::ChatError { .. } => "chat_error",
            Self::ChatCancelled { .. } => "chat_cancelled",
            Self::ChannelEvent { .. } => "channel_event",
        }
    }
}
