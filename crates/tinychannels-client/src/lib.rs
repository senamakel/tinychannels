//! Typed host calls to confidential TinyChannels module operations.

mod client;
mod runtime;
mod types;

pub use client::TinyChannelsClient;
pub use runtime::{ChannelRuntimeClient, ChannelRuntimeDriver};
pub use types::{ChannelRuntimeError, ChannelRuntimeErrorKind, ChannelRuntimeOperation};

#[derive(Clone, Copy)]
pub(crate) enum RuntimeStart {
    Serial,
    Dispatch(tinychannels_bus::ChannelRuntimeDispatchPolicy),
}

#[cfg(test)]
#[path = "lib_tests.rs"]
mod tests;
