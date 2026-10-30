//! In-process provider seams and legacy send helpers.
use async_trait::async_trait;
pub use tinychannels_bus::traits::{ChannelMessage, SendMessage};

/// In-process provider operations implemented by TinyChannels backends.
///
/// This trait belongs to the runtime API because it includes asynchronous
/// provider operations and Tokio listener channels. The bus contract only
/// carries the serializable message DTOs.
#[async_trait]
pub trait Channel: Send + Sync {
    fn name(&self) -> &str;
    fn proactive_target(&self) -> Option<String> {
        None
    }
    async fn send(&self, message: &SendMessage) -> anyhow::Result<()>;
    async fn listen(&self, tx: tokio::sync::mpsc::Sender<ChannelMessage>) -> anyhow::Result<()>;
    async fn health_check(&self) -> bool {
        true
    }
    async fn start_typing(&self, _recipient: &str) -> anyhow::Result<()> {
        Ok(())
    }
    async fn stop_typing(&self, _recipient: &str) -> anyhow::Result<()> {
        Ok(())
    }
    async fn begin_typing_refresh(
        &self,
        _recipient: &str,
        _interval_secs: u64,
    ) -> anyhow::Result<Option<String>> {
        Ok(None)
    }
    async fn end_typing_refresh(&self, _handle: &str) -> anyhow::Result<()> {
        Ok(())
    }
    fn capabilities(&self) -> tinychannels_bus::ChannelCapabilities {
        tinychannels_bus::capabilities::capabilities_for(self.name())
    }
    fn supports_reactions(&self) -> bool {
        false
    }
    fn supports_draft_updates(&self) -> bool {
        false
    }
    async fn send_draft(&self, _message: &SendMessage) -> anyhow::Result<Option<String>> {
        Ok(None)
    }
    async fn update_draft(
        &self,
        _recipient: &str,
        _message_id: &str,
        _text: &str,
    ) -> anyhow::Result<()> {
        Ok(())
    }
    async fn finalize_draft(
        &self,
        _recipient: &str,
        _message_id: &str,
        _text: &str,
        _thread_ts: Option<&str>,
    ) -> anyhow::Result<()> {
        Ok(())
    }
    async fn send_idempotent(&self, message: &SendMessage) -> anyhow::Result<()> {
        self.send(message).await
    }
}

#[async_trait]
pub trait ChannelSendExt: Channel {
    async fn send_with_outbound_intent(&self, message: &SendMessage) -> anyhow::Result<()> {
        Channel::send_idempotent(self, message).await
    }
}
impl<T: Channel + ?Sized> ChannelSendExt for T {}

/// Runtime idempotency construction for the legacy send DTO.
pub trait SendMessageExt {
    fn with_deterministic_idempotency_key(self, channel_id: impl AsRef<str>) -> Self;
}
impl SendMessageExt for SendMessage {
    /// Generate a deterministic idempotency key from this legacy send shape.
    fn with_deterministic_idempotency_key(mut self, channel_id: impl AsRef<str>) -> Self {
        let intent = crate::channel::outbound_intent_from_send_message(channel_id.as_ref(), &self);
        self.idempotency_key = Some(intent.idempotency_key);
        self
    }
}

#[cfg(test)]
#[path = "traits_tests.rs"]
mod tests;
