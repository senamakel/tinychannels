//! Portable typed content and voice-note payloads.
use super::envelope::MediaReference;
use super::capabilities::MarkdownDialect;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use serde_json::Value;

/// Content attached to an inbound channel message or chat request.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum ContentPart {
    Text { text: String, markdown_dialect: Option<MarkdownDialect> },
    Image { media: MediaReference, caption: Option<String>, alt_text: Option<String> },
    Video { media: MediaReference, caption: Option<String>, duration_ms: Option<u64>, thumbnail: Option<MediaReference> },
    Audio { media: MediaReference, codec: Option<String>, duration_ms: Option<u64>, title: Option<String>, transcript: Option<String> },
    File { media: MediaReference, file_name: Option<String>, caption: Option<String> },
    VoiceNote { voice_note: VoiceNote },
    Sticker { media: MediaReference, pack_id: Option<String>, sticker_id: Option<String>, emoji: Option<String>, alt_text: Option<String> },
    Location { latitude: f64, longitude: f64, name: Option<String>, address: Option<String> },
    Contact { name: String, phone_numbers: Vec<String>, emails: Vec<String> },
    Buttons { text: String, buttons: Vec<Value> },
    Poll { question: String, options: Vec<String> },
    Presentation { data: Value },
    Native { data: Value },
}

/// Voice media sent as a typed channel output.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum OutboundVoice {
    Media { media: MediaReference, codec: Option<String>, duration_ms: Option<u64> },
}

/// Ordered content part sent to a channel provider.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum OutboundContentPart {
    Content { part: ContentPart },
    Voice { voice: OutboundVoice },
}

/// Durable reference to an inbound voice note.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(default, rename_all = "camelCase")]
pub struct VoiceNote {
    pub media: MediaReference,
    pub transcript: Option<VoiceTranscript>,
    pub codec: Option<String>,
    pub duration_ms: Option<u64>,
    pub waveform: Option<Vec<f32>>,
}

/// Transcript attached to or derived from an audio note.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct VoiceTranscript {
    pub text: String,
    pub language: Option<String>,
    pub state: VoiceTranscriptState,
    pub source: VoiceTranscriptSource,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum VoiceTranscriptState { Pending, Complete, Failed }

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum VoiceTranscriptSource { Provider, HostStt, User }
