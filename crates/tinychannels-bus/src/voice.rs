//! Portable voice callback payloads. Provider selection and speech processing
//! remain owned by the host.
use crate::{ChannelRef, ConversationRef};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use serde_json::Value;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum HostVoiceReplyMode {
    Mirror,
    Always,
    Never,
    SpeakReply,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum HostVoiceMediaDirection {
    Inbound,
    Outbound,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(default, rename_all = "camelCase")]
pub struct HostVoiceScope {
    pub configured_instance: String,
    pub channel: ChannelRef,
    pub conversation: ConversationRef,
    pub turn_id: String,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(default, rename_all = "camelCase")]
pub struct HostTranscriptionRequest {
    pub audio_base64: String,
    pub mime_type: Option<String>,
    pub file_name: Option<String>,
    pub language: Option<String>,
    pub scope: Option<HostVoiceScope>,
}

#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(default, rename_all = "camelCase")]
pub struct HostTranscriptionResult {
    pub text: String,
    pub language: Option<String>,
    pub duration_secs: Option<f64>,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(default, rename_all = "camelCase")]
pub struct HostVoiceTranscriptionRequest {
    pub scope: HostVoiceScope,
    pub audio_base64: String,
    pub mime_type: Option<String>,
    pub file_name: Option<String>,
    pub language: Option<String>,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(default, rename_all = "camelCase")]
pub struct HostSpeechSynthesisRequest {
    pub scope: HostVoiceScope,
    pub text: String,
    pub voice: Option<String>,
    pub format: Option<String>,
}

#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(default, rename_all = "camelCase")]
pub struct HostSpeechSynthesisResult {
    pub audio_base64: String,
    pub mime_type: String,
    pub duration_ms: Option<u64>,
    pub visemes: Option<Value>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct HostVoiceReplyPolicyRequest {
    pub scope: HostVoiceScope,
    pub inbound_has_voice: bool,
    pub explicitly_requested: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct HostVoiceReplyPolicy {
    pub mode: HostVoiceReplyMode,
}
