//! Outbound channel intent types.

use crate::channel::envelope::{ContentPart, MediaReference};
use crate::channel::types::{ChannelRef, ConversationRef};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use serde_json::Value;

/// Delivery durability requested by core when sending agent output.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum DeliveryDurability {
    Required,
    #[default]
    BestEffort,
    Disabled,
}

/// Provider-neutral outbound payload variants.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum OutboundPayload {
    Text {
        text: String,
    },
    Media {
        text: Option<String>,
        media_urls: Vec<String>,
    },
    Voice {
        media_url: String,
    },
    Parts {
        parts: Vec<OutboundContentPart>,
    },
    Files {
        file_urls: Vec<String>,
    },
    Poll {
        question: String,
        options: Vec<String>,
    },
    PresentationBlocks {
        blocks: Value,
    },
    NativeChannelData {
        data: Value,
    },
}

/// Explicit typed voice payload.
#[allow(
    clippy::large_enum_variant,
    reason = "typed voice wire variants stay unboxed for ergonomic serialization"
)]
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(
    tag = "kind",
    rename_all = "snake_case",
    rename_all_fields = "camelCase"
)]
pub enum OutboundVoice {
    Media {
        media: MediaReference,
        codec: Option<String>,
        duration_ms: Option<u64>,
    },
    Synthesize {
        text: String,
        voice: Option<String>,
    },
}

/// Ordered provider-neutral content accepted by typed channel adapters.
#[allow(
    clippy::large_enum_variant,
    reason = "typed content wire variants stay unboxed to preserve the public parts API"
)]
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(
    tag = "kind",
    rename_all = "snake_case",
    rename_all_fields = "camelCase"
)]
pub enum OutboundContentPart {
    Content { part: ContentPart },
    Voice { voice: OutboundVoice },
}

/// Complete provider-facing send, retaining structured content and addressing.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct ChannelOutboundMessage {
    pub channel_id: String,
    pub conversation_id: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub channel_ref: Option<ChannelRef>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub conversation_ref: Option<ConversationRef>,
    pub reply_to_id: Option<String>,
    pub thread_id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub turn_id: Option<String>,
    pub idempotency_key: String,
    pub parts: Vec<OutboundContentPart>,
}

impl ChannelOutboundMessage {
    /// Adapt legacy payload variants to ordered typed parts without data loss.
    pub fn from_intent(intent: &ChannelOutboundIntent) -> Self {
        use crate::channel::envelope::MediaKind;
        let mut parts = Vec::new();
        match &intent.payload {
            OutboundPayload::Text { text } => parts.push(text_part(text.clone())),
            OutboundPayload::Media { text, media_urls } => {
                if let Some(text) = text {
                    parts.push(text_part(text.clone()));
                }
                parts.extend(media_urls.iter().map(|url| OutboundContentPart::Content {
                    part: ContentPart::Image {
                        media: MediaReference {
                            url: Some(url.clone()),
                            kind: MediaKind::Image,
                            ..Default::default()
                        },
                        caption: None,
                        alt_text: None,
                    },
                }));
            }
            OutboundPayload::Voice { media_url } => parts.push(OutboundContentPart::Voice {
                voice: OutboundVoice::Media {
                    media: MediaReference {
                        url: Some(media_url.clone()),
                        kind: MediaKind::Audio,
                        ..Default::default()
                    },
                    codec: None,
                    duration_ms: None,
                },
            }),
            OutboundPayload::Files { file_urls } => {
                parts.extend(file_urls.iter().map(|url| OutboundContentPart::Content {
                    part: ContentPart::File {
                        media: MediaReference {
                            url: Some(url.clone()),
                            kind: MediaKind::Document,
                            ..Default::default()
                        },
                        file_name: None,
                        caption: None,
                    },
                }))
            }
            OutboundPayload::Poll { question, options } => {
                parts.push(OutboundContentPart::Content {
                    part: ContentPart::Poll {
                        question: question.clone(),
                        options: options.clone(),
                        multiple_choice: false,
                        closes_at_unix_ms: None,
                        provider_poll_id: None,
                    },
                })
            }
            OutboundPayload::PresentationBlocks { blocks } => {
                parts.push(OutboundContentPart::Content {
                    part: ContentPart::Presentation {
                        blocks: blocks.clone(),
                    },
                })
            }
            OutboundPayload::NativeChannelData { data } => {
                parts.push(OutboundContentPart::Content {
                    part: ContentPart::Native {
                        provider: intent.channel_id.clone(),
                        data: data.clone(),
                    },
                })
            }
            OutboundPayload::Parts { parts: supplied } => parts.extend(supplied.iter().cloned()),
        }
        let conversation_ref = intent.conversation_ref.clone().map(|mut r| {
            if r.thread_id.is_none() {
                r.thread_id.clone_from(&intent.thread_id);
            }
            r
        });
        Self {
            channel_id: intent
                .channel_ref
                .as_ref()
                .map(|r| r.id.clone())
                .unwrap_or_else(|| intent.channel_id.clone()),
            conversation_id: intent
                .conversation_ref
                .as_ref()
                .map(|r| r.id.clone())
                .unwrap_or_else(|| intent.conversation_id.clone()),
            channel_ref: intent.channel_ref.clone(),
            thread_id: intent.thread_id.clone().or_else(|| {
                intent
                    .conversation_ref
                    .as_ref()
                    .and_then(|r| r.thread_id.clone())
            }),
            conversation_ref,
            reply_to_id: intent.reply_to_id.clone(),
            turn_id: intent.turn_id.clone(),
            idempotency_key: intent.idempotency_key.clone(),
            parts,
        }
    }

    /// Return a text-only compatibility representation, rejecting all rich content.
    pub fn legacy_text(&self) -> anyhow::Result<String> {
        let mut text = Vec::new();
        for part in &self.parts {
            match part {
                OutboundContentPart::Content {
                    part:
                        ContentPart::Text {
                            text: value,
                            markdown_dialect: None,
                        },
                } => text.push(value.as_str()),
                _ => anyhow::bail!("provider does not support typed channel content"),
            }
        }
        Ok(text.join("\n"))
    }

    /// Convert a typed message to the legacy text send contract when lossless.
    pub fn to_legacy_message(&self) -> anyhow::Result<crate::traits::SendMessage> {
        Ok(crate::traits::SendMessage {
            content: self.legacy_text()?,
            recipient: self
                .conversation_ref
                .as_ref()
                .map(|r| r.id.clone())
                .unwrap_or_else(|| self.conversation_id.clone()),
            subject: None,
            thread_ts: self.thread_id.clone().or_else(|| {
                self.conversation_ref
                    .as_ref()
                    .and_then(|r| r.thread_id.clone())
            }),
            idempotency_key: (!self.idempotency_key.is_empty())
                .then(|| self.idempotency_key.clone()),
        })
    }
}

fn text_part(text: String) -> OutboundContentPart {
    OutboundContentPart::Content {
        part: ContentPart::Text {
            text,
            markdown_dialect: None,
        },
    }
}

#[cfg(test)]
#[path = "intent_tests.rs"]
mod tests;

/// Logical outbound send request.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct ChannelOutboundIntent {
    pub idempotency_key: String,
    pub channel_id: String,
    pub conversation_id: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub channel_ref: Option<ChannelRef>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub conversation_ref: Option<ConversationRef>,
    pub reply_to_id: Option<String>,
    pub thread_id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub turn_id: Option<String>,
    pub durability: DeliveryDurability,
    pub payload: OutboundPayload,
}

impl Default for ChannelOutboundIntent {
    fn default() -> Self {
        Self {
            idempotency_key: String::new(),
            channel_id: String::new(),
            conversation_id: String::new(),
            channel_ref: None,
            conversation_ref: None,
            reply_to_id: None,
            thread_id: None,
            turn_id: None,
            durability: DeliveryDurability::BestEffort,
            payload: OutboundPayload::Text {
                text: String::new(),
            },
        }
    }
}
