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
//! `tinymemory`'s `RuntimeHost` uses, and it is why `tinychannels-bus` declares
//! two objects rather than one.
//!
//! # One ordered consumer, not a task per message
//!
//! Notifications go through a bounded queue drained by a **single** task that
//! awaits each call before starting the next. Two properties fall out, and both
//! are load-bearing:
//!
//! - **Order is preserved.** Spawning a task per notification would let two host
//!   calls race, and the host would observe conversation turns in an order the
//!   provider never produced them in. For a chat surface that is corruption, not
//!   a performance detail.
//! - **Memory is bounded.** A task per notification also means unbounded
//!   outstanding calls and cloned payloads whenever the host is slower than the
//!   traffic, which is exactly when you can least afford to grow.
//!
//! Backpressure therefore propagates the way it already did: a full outbox
//! blocks the forwarder, which stops draining the provider's bounded inbound
//! queue, which blocks the provider — the same path a slow consumer always took.
//! Nothing is silently dropped.
//!
//! # Failures are logged, not retried
//!
//! A failed delivery is logged and the queue moves on. Retrying here would be
//! wrong: this module has no durable spool, so a retry loop would either stall
//! the ordered queue behind one unreachable host or grow in memory. Durable
//! redelivery belongs to whoever owns storage, which is the host.

use tinybus::Connection;
use tinychannels_bus::names::{HOST_BUS_NAME, HOST_OBJECT_PATH, host_methods};
use tokio::sync::mpsc;

/// Bound on notifications waiting to be handed to the host.
///
/// Sized to match the per-provider inbound queue in `service.rs`, so the two
/// buffers cannot combine into a surprise multiple of either.
const OUTBOX_CAPACITY: usize = 256;

/// One queued host call, already serialised into its argument tuple.
enum Notification {
    DeliverInbound {
        channel: String,
        message: serde_json::Value,
    },
    ReportStatus {
        channel: String,
        state: String,
        detail: Option<String>,
    },
}

/// Client for the callback object the host serves.
#[derive(Clone)]
pub struct HostChannels {
    outbox: mpsc::Sender<Notification>,
}

impl std::fmt::Debug for HostChannels {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("HostChannels")
            .finish_non_exhaustive()
    }
}

impl HostChannels {
    /// Wrap a connection and start the ordered consumer.
    #[must_use]
    pub fn new(connection: Connection) -> Self {
        let (outbox, mut inbox) = mpsc::channel(OUTBOX_CAPACITY);
        tokio::spawn(async move {
            while let Some(notification) = inbox.recv().await {
                // Resolved per notification rather than once here: a host may
                // start serving this object after the module loads, and a name
                // captured up front would pin the failure forever.
                let proxy = match connection.proxy(HOST_BUS_NAME, HOST_OBJECT_PATH, HOST_BUS_NAME) {
                    Ok(proxy) => proxy,
                    Err(error) => {
                        log::debug!("[tinychannels:module] host callback unreachable: {error}");
                        continue;
                    }
                };
                // Awaited, not spawned — this is what keeps the order.
                let result = match notification {
                    Notification::DeliverInbound { channel, message } => {
                        proxy
                            .call::<()>(host_methods::DELIVER_INBOUND, (channel, message))
                            .await
                    }
                    Notification::ReportStatus {
                        channel,
                        state,
                        detail,
                    } => {
                        proxy
                            .call::<()>(host_methods::REPORT_STATUS, (channel, state, detail))
                            .await
                    }
                };
                if let Err(error) = result {
                    // `debug`, not `warn`: a host that serves no callback object
                    // is a supported configuration (outbound-only), and warning
                    // per received message would be a log flood, not a signal.
                    log::debug!("[tinychannels:module] host callback failed: {error}");
                }
            }
        });
        Self { outbox }
    }

    /// Hand the host one inbound message.
    pub async fn deliver_inbound(&self, channel: &str, message: serde_json::Value) {
        self.enqueue(Notification::DeliverInbound {
            channel: channel.to_owned(),
            message,
        })
        .await;
    }

    /// Report a provider connection-state transition.
    ///
    /// Goes through the same queue as inbound messages so a `stopped` cannot
    /// overtake the last message that arrived before it.
    pub async fn report_status(&self, channel: &str, state: &str, detail: Option<String>) {
        self.enqueue(Notification::ReportStatus {
            channel: channel.to_owned(),
            state: state.to_owned(),
            detail,
        })
        .await;
    }

    async fn enqueue(&self, notification: Notification) {
        // Only fails once the consumer is gone, which means the module is
        // shutting down; there is nothing useful left to do with the payload.
        if self.outbox.send(notification).await.is_err() {
            log::debug!("[tinychannels:module] host callback consumer has stopped");
        }
    }
}
