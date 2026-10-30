//! Pure serialized vocabulary, schema declarations and errors for TinyChannels.
//! Provider seams and algorithms live in `tinychannels-runtime`; the root
//! implementation crate preserves legacy module paths through re-exports.
pub mod approvals;
pub mod capabilities;
pub mod channel;
pub mod config;
pub mod context;
pub mod controllers;
pub mod error;
pub mod health;
pub mod host;
pub mod http;
pub mod module_control;
pub mod module_controller;
pub mod module_delivery;
pub mod module_email;
pub mod module_inbound;
pub mod module_interaction;
pub mod module_media;
pub mod module_progressive;
pub mod module_relay;
pub mod module_remote;
pub mod module_routes;
pub mod module_runtime;
pub mod module_supervision;
pub mod module_web_output;
pub mod names;
pub mod relay;
pub mod storage;
pub mod storage_names;
pub mod traits;
pub mod version;
pub mod voice;
pub mod web;
pub use capabilities::{ChannelCapabilities, capabilities_for, provider_id};
pub use channel::content::{
    ContentPart, OutboundContentPart, OutboundVoice, VoiceNote, VoiceTranscript,
    VoiceTranscriptSource, VoiceTranscriptState,
};
pub use channel::envelope::{MediaKind, MediaReference};
pub use channel::event::{ChannelEvent, ChannelEventKind, ChannelReaction};
pub use channel::types::{ChannelRef, ConversationKind, ConversationRef, SenderRef};
pub use channel::{
    ChannelInboundEnvelope, ChannelOutboundIntent, DeliveryDurability, OutboundPayload,
};
pub use config::ChannelsConfig;
pub use controllers::{ChannelAuthMode, ChannelDefinition};
pub use error::{Result, TinyChannelsError};
pub use health::{ChannelHealthReport, ChannelHealthState};
pub use host::{HostEventPublication, HostTranscriptionRequest, HostTranscriptionResult};
pub use http::HttpProxyPolicy;
pub use module_controller::{
    ChannelBackendRequest, ChannelBackendResult, ChannelControllerRequest, ChannelControllerResult,
};
pub use module_delivery::{DeliveryQueueEntry, DeliveryRecoveryState, DeliveryTerminalResult};
pub use module_email::{
    EmailAttachment, EmailAttachmentUpload, EmailSendRequest, EmailUploadBegin, EmailUploadChunk,
    EmailUploadHandle,
};
pub use module_media::{
    MediaReadChunkResult, MediaReadHandle, MediaReadInfo, MediaUploadBegin, MediaUploadChunk,
    MediaUploadFinish, MediaUploadHandle,
};
pub use names::{BUS_NAME, HOST_BUS_NAME, HOST_OBJECT_PATH, METHODS, OBJECT_PATH, methods};
pub use traits::{Channel, ChannelMessage, ChannelSendExt, SendMessage};
pub use version::{CONTRACT_VERSION, is_compatible};
pub use voice::{
    HostSpeechSynthesisRequest, HostSpeechSynthesisResult, HostVoiceMediaDirection,
    HostVoiceReplyMode, HostVoiceReplyPolicy, HostVoiceReplyPolicyRequest, HostVoiceScope,
    HostVoiceTranscriptionRequest,
};

#[cfg(test)]
#[path = "wire_tests.rs"]
mod tests;

pub use module_interaction::{
    ChannelInteraction, ChannelInteractionInfo, ChannelInteractionRequest, ChannelInteractionResult,
};

pub use module_supervision::ChannelSupervisionPolicy;

pub use module_runtime::{ChannelRuntimeDispatchPolicy, ChannelRuntimeReservation};
