//! TinyBus wire contract for TinyChannels: the call vocabulary a host and the
//! channels module share.
//!
//! This crate is deliberately transport-free and dependency-light. It owns the
//! types that cross the boundary — inbound envelopes, outbound intents, channel
//! configuration, controller metadata, relay frames and the session-key rules —
//! and nothing that opens a socket, spawns a task or touches a database.
//!
//! # Why the split is here and not elsewhere
//!
//! A host that only needs to *name* a channel message pays for serde and
//! nothing more; the provider stack (Telegram, Discord, Slack, IMAP, …) with
//! its `reqwest`, `rusqlite`, `rustls` and `tokio-tungstenite` dependencies
//! lives in the `tinychannels` crate and is reached over the bus.
//!
//! Two rules follow from that, and both are load-bearing:
//!
//! - **Never re-declare a contract type in a host.** A field added on one side
//!   of a copy is a decode failure on the other with nothing to catch it.
//! - **Call members by their constant**, never by a string literal —
//!   [`methods::SEND_MESSAGE`], not `"SendMessage"`. A rename upstream is then a
//!   compile error rather than a `MemberNotFound` in the field.
//!
//! # Session keys are contract, not policy
//!
//! [`build_session_key_for_inbound_envelope`] derives a **persisted** grouping
//! key that a host writes to its conversation store. It lives here for the same
//! reason `tinywallet-bus` owns address validation: both sides must agree
//! exactly, and a drifted copy silently regroups existing user data.

pub mod adapters;
pub mod channel;
pub mod config;
pub mod context;
pub mod controllers;
pub mod error;
pub mod names;
pub mod relay;
pub mod security;
pub mod text;
pub mod traits;
pub mod version;

pub use channel::{
    ChannelInboundEnvelope, ChannelOutboundIntent, DeliveryDurability, OutboundPayload,
    build_session_key_for_inbound_envelope, inbound_envelope_from_legacy_message,
    legacy_message_from_inbound_envelope, legacy_message_value_from_outbound_intent,
    outbound_intent_from_legacy_message, outbound_intent_from_send_message,
};
pub use config::ChannelsConfig;
pub use controllers::{ChannelAuthMode, ChannelDefinition};
pub use error::{Result, TinyChannelsError};
pub use names::{BUS_NAME, HOST_BUS_NAME, HOST_OBJECT_PATH, METHODS, OBJECT_PATH, methods};
pub use traits::{Channel, ChannelMessage, ChannelSendExt, SendMessage};
pub use version::{CONTRACT_VERSION, is_compatible};
