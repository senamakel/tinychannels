//! The host-side object this module calls, and the client that reaches it.
//!
//! # Why inbound needs its own object
//!
//! `Interface::call` receives a member name and a JSON body — no caller
//! identity, no connection — so a served object cannot open a stream or send a
//! reply back to whoever called it. That is fine for a codec, where every
//! answer belongs to the call that asked for it. A channel is not a codec: the
//! interesting traffic is unsolicited, arriving from Telegram or Slack at a
//! moment no host call is outstanding.
//!
//! So the direction is inverted for that half. The **host** serves
//! [`HOST_BUS_NAME`], and this module calls it. That is the same shape
//! `tinymemory`'s `RuntimeHost` uses, and it is why
//! `tinychannels-bus` declares two objects rather than one.
//!
//! # Delivery is fire-and-forget, deliberately
//!
//! [`HostChannels::deliver_inbound`] spawns and does not await the host's
//! answer. A provider's listen loop is a hot path with no useful response to
//! wait for, and blocking it on the host would let a slow host apply
//! backpressure all the way into a third party's socket — where the failure
//! mode is a dropped connection, not a queued message.
//!
//! The cost is that a failed delivery is logged, not retried. Retrying here
//! would be wrong anyway: this module has no durable spool, so a retry loop
//! would either block the listen task or grow unboundedly in memory. Durable
//! redelivery belongs to whoever owns storage, which is the host.

use tinybus::Connection;
use tinychannels_bus::names::{HOST_BUS_NAME, HOST_OBJECT_PATH, host_methods};

/// Client for the callback object the host serves.
#[derive(Clone)]
pub struct HostChannels {
    connection: Connection,
}

impl std::fmt::Debug for HostChannels {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.debug_struct("HostChannels").finish_non_exhaustive()
    }
}

impl HostChannels {
    /// Wrap a connection.
    #[must_use]
    pub fn new(connection: Connection) -> Self {
        Self { connection }
    }

    fn proxy(&self) -> Result<tinybus::Proxy, tinybus::Error> {
        self.connection
            .proxy(HOST_BUS_NAME, HOST_OBJECT_PATH, HOST_BUS_NAME)
    }

    /// Hand the host one inbound message.
    ///
    /// Resolution happens per call rather than once at setup: a host may serve
    /// this object after the module loads, and a name captured at setup would
    /// pin the failure forever.
    pub fn deliver_inbound(&self, channel: &str, message: &serde_json::Value) {
        self.notify(
            host_methods::DELIVER_INBOUND,
            (channel.to_owned(), message.clone()),
        );
    }

    /// Report a provider connection-state transition.
    pub fn report_status(&self, channel: &str, state: &str, detail: Option<String>) {
        self.notify(
            host_methods::REPORT_STATUS,
            (channel.to_owned(), state.to_owned(), detail),
        );
    }

    fn notify<T>(&self, method: &'static str, arguments: T)
    where
        T: serde::Serialize + Send + 'static,
    {
        let host = self.clone();
        tokio::spawn(async move {
            let result = match host.proxy() {
                Ok(proxy) => proxy.call::<()>(method, arguments).await,
                Err(error) => Err(error),
            };
            if let Err(error) = result {
                // `debug`, not `warn`: a host that serves no callback object is
                // a supported configuration (outbound-only), and warning on
                // every received message would be a log flood, not a signal.
                log::debug!("[tinychannels:module] host callback {method} failed: {error}");
            }
        });
    }
}
